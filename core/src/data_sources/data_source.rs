use crate::consts::DATA_SOURCE_DOCUMENT_SYSTEM_TAG_PREFIX;
use crate::data_sources::splitter::{splitter, SplitterID};
use crate::project::Project;
use crate::providers::embedder::{EmbedderRequest, EmbedderVector};
use crate::providers::provider::{provider, ProviderID};
use crate::run::Credentials;
use crate::stores::{sqlite::SQLiteStore, store::Store};
use crate::utils;
use anyhow::{anyhow, Result};
use cloud_storage::Object;
use futures::try_join;
use futures::StreamExt;
use futures::TryStreamExt;
use qdrant_client::qdrant::{points_selector::PointsSelectorOneOf, Filter, PointsSelector};
use qdrant_client::{
    prelude::{Payload, QdrantClient, QdrantClientConfig},
    qdrant,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// A filter to apply to the search query based on `tags`. All documents returned must have at list
/// one tag in `is_in` and none of the tags in `is_not`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TagsFilter {
    #[serde(rename = "in")]
    pub is_in: Option<Vec<String>>,
    #[serde(rename = "not")]
    pub is_not: Option<Vec<String>>,
}

/// A filter to apply to the search query based on `timestamp`. All documents returned must have a
/// timestamp greater than `gt` and less than `lt`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TimestampFilter {
    pub gt: Option<u64>,
    pub lt: Option<u64>,
}

/// Filter argument to perform semantic search. It is used to filter the search results based on the
/// presence of tags or time spans for timestamps.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchFilter {
    pub tags: Option<TagsFilter>,
    pub timestamp: Option<TimestampFilter>,
}

impl SearchFilter {
    pub fn from_json_str(json: &str) -> Result<Self> {
        let filter: SearchFilter = serde_json::from_str(json)?;
        Ok(filter)
    }
}

/// A Chunk is a subset of a document that was inserted into vector search db. `hash` covers both
/// the chunk text and the parent document tags (inserted into vector db search on each chunk to
/// leverage tags filtering there). It is used as unique ID for the chunk in vector search db.
#[derive(Debug, Serialize, Clone)]
pub struct Chunk {
    pub text: String,
    pub hash: String,
    pub offset: usize,
    pub vector: Option<Vec<f64>>,
    pub score: Option<f64>,
}

/// Document is used as a data-strucutre for insertion into the SQL store (no chunks, they are
/// directly inserted in the vector search db). It is also used as a result from search (only the
/// retrieved chunks are provided in the result). `hash` covers both the original document id and
/// text and the document metadata and is used to no-op in case of match.
#[derive(Debug, Serialize, Clone)]
pub struct Document {
    pub data_source_id: String,
    pub created: u64,
    pub document_id: String,
    pub timestamp: u64,
    pub tags: Vec<String>,
    pub source_url: Option<String>,
    pub hash: String,
    pub text_size: u64,
    pub chunk_count: usize,
    pub chunks: Vec<Chunk>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    pub token_count: Option<usize>,
}

impl Document {
    pub fn new(
        data_source_id: &str,
        document_id: &str,
        timestamp: u64,
        tags: &Vec<String>,
        source_url: &Option<String>,
        hash: &str,
        text_size: u64,
    ) -> Result<Self> {
        Ok(Document {
            data_source_id: data_source_id.to_string(),
            created: utils::now(),
            document_id: document_id.to_string(),
            timestamp,
            tags: tags.clone(),
            source_url: source_url.clone(),
            hash: hash.to_string(),
            text_size,
            chunk_count: 0,
            chunks: vec![],
            text: None,
            token_count: None,
        })
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct DocumentVersion {
    pub created: u64,
    pub hash: String,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct DataSourceConfig {
    pub provider_id: ProviderID,
    pub model_id: String,
    pub extras: Option<Value>,
    pub splitter_id: SplitterID,
    pub max_chunk_size: usize,
    pub use_cache: bool,
}

/// The `data_source_id` is the unique identifier that allows routing to the right data in SQL store
/// as well as vector search db. It is a generated unique ID.
#[derive(Debug, Serialize)]
pub struct DataSource {
    project: Project,
    created: u64,
    data_source_id: String,
    internal_id: String,
    config: DataSourceConfig,
}

fn target_document_tokens_offsets(
    offsets: Vec<usize>,
    chunks_to_grow: usize,
    total_chunks_count: usize,
) -> HashMap<usize, usize> {
    // Note: we could increment num_addable when we don't get enough chunks on a given chunks to cram more chunks
    if total_chunks_count == 0 {
        return HashMap::new();
    }
    let mut offsets = offsets;
    offsets.sort();
    let mut offset_set = offsets
        .clone()
        .into_iter()
        .collect::<std::collections::HashSet<_>>();
    let mut results: HashMap<usize, usize> = HashMap::new();
    let mut extras: Vec<(usize, usize)> = vec![];
    let num_per_chunk = chunks_to_grow / offsets.len();
    for i in 0..offsets.len() {
        let cur_extra_right = if i == offsets.len() - 1 {
            total_chunks_count - offsets[i] - 1
        } else {
            offsets[i + 1] - offsets[i] - 1
        };
        let cur_extra_left = if i == 0 {
            offsets[i]
        } else {
            offsets[i] - (offsets[i - 1] + 1 + extras[i - 1].1)
        };
        if cur_extra_left >= num_per_chunk / 2 && cur_extra_right >= num_per_chunk / 2 {
            extras.push((num_per_chunk / 2, num_per_chunk / 2));
        } else if (cur_extra_left + cur_extra_right) < num_per_chunk {
            extras.push((cur_extra_left, cur_extra_right));
        } else if cur_extra_left < cur_extra_right {
            extras.push((cur_extra_left, num_per_chunk - cur_extra_left));
        } else {
            extras.push((num_per_chunk - cur_extra_right, cur_extra_right));
        }
    }
    for i in 0..offsets.len() {
        let (cur_extra_left, cur_extra_right) = extras[i];
        for offset in offsets[i] - cur_extra_left..offsets[i] + cur_extra_right + 1 {
            if !offset_set.contains(&offset) {
                results.insert(offset, offsets[i]);
                offset_set.insert(offset);
            }
        }
    }
    results
}

impl DataSource {
    pub fn new(project: &Project, data_source_id: &str, config: &DataSourceConfig) -> Self {
        DataSource {
            project: project.clone(),
            created: utils::now(),
            data_source_id: data_source_id.to_string(),
            internal_id: utils::new_id(),
            config: config.clone(),
        }
    }

    pub fn new_from_store(
        project: &Project,
        created: u64,
        data_source_id: &str,
        internal_id: &str,
        config: &DataSourceConfig,
    ) -> Self {
        DataSource {
            project: project.clone(),
            created,
            data_source_id: data_source_id.to_string(),
            internal_id: internal_id.to_string(),
            config: config.clone(),
        }
    }

    pub fn created(&self) -> u64 {
        self.created
    }

    pub fn data_source_id(&self) -> &str {
        &self.data_source_id
    }

    pub fn internal_id(&self) -> &str {
        &self.internal_id
    }

    pub fn config(&self) -> &DataSourceConfig {
        &self.config
    }

    fn qdrant_collection(&self) -> String {
        format!("ds_{}", self.internal_id)
    }

    async fn qdrant_client(&self) -> Result<QdrantClient> {
        match std::env::var("QDRANT_URL") {
            Ok(url) => {
                let mut config = QdrantClientConfig::from_url(&url);
                match std::env::var("QDRANT_API_KEY") {
                    Ok(api_key) => {
                        config.set_api_key(&api_key);
                        QdrantClient::new(Some(config))
                    }
                    Err(_) => Err(anyhow!("QDRANT_API_KEY is not set"))?,
                }
            }
            Err(_) => Err(anyhow!("QDRANT_URL is not set"))?,
        }
    }

    pub async fn setup(&self, credentials: Credentials) -> Result<()> {
        let mut embedder = provider(self.config.provider_id).embedder(self.config.model_id.clone());
        embedder.initialize(credentials).await?;

        // GCP store created data to test GCP.
        let bucket = match std::env::var("DUST_DATA_SOURCES_BUCKET") {
            Ok(bucket) => bucket,
            Err(_) => Err(anyhow!("DUST_DATA_SOURCES_BUCKET is not set"))?,
        };

        let bucket_path = format!("{}/{}", self.project.project_id(), self.internal_id);
        let data_source_created_path = format!("{}/created.txt", bucket_path);

        Object::create(
            &bucket,
            format!("{}", self.created).as_bytes().to_vec(),
            &data_source_created_path,
            "application/text",
        )
        .await?;

        utils::done(&format!(
            "Created GCP bucket for data_source `{}`",
            self.data_source_id
        ));

        // Qdrant create collection.
        let qdrant_client = self.qdrant_client().await?;
        qdrant_client
            .create_collection(&qdrant::CreateCollection {
                collection_name: self.qdrant_collection(),
                vectors_config: Some(qdrant::VectorsConfig {
                    config: Some(qdrant::vectors_config::Config::Params(
                        qdrant::VectorParams {
                            size: embedder.embedding_size() as u64,
                            distance: qdrant::Distance::Cosine.into(),
                            ..Default::default()
                        },
                    )),
                }),
                hnsw_config: Some(qdrant::HnswConfigDiff {
                    m: Some(16),
                    ..Default::default()
                }),
                optimizers_config: Some(qdrant::OptimizersConfigDiff {
                    memmap_threshold: Some(1024),
                    ..Default::default()
                }),
                // We keep the entire payload on disk and index on document_id and tags.
                on_disk_payload: Some(true),
                ..Default::default()
            })
            .await?;

        let _ = qdrant_client
            .create_field_index(
                self.qdrant_collection(),
                "document_id_hash",
                qdrant::FieldType::Keyword,
                None,
                None,
            )
            .await?;

        let _ = qdrant_client
            .create_field_index(
                self.qdrant_collection(),
                "tags",
                qdrant::FieldType::Keyword,
                None,
                None,
            )
            .await?;

        let _ = qdrant_client
            .create_field_index(
                self.qdrant_collection(),
                "timestamp",
                qdrant::FieldType::Integer,
                None,
                None,
            )
            .await?;

        utils::done(&format!(
            "Created Qdrant collection and indexes for data_source `{}`",
            self.data_source_id
        ));

        Ok(())
    }

    pub async fn update_tags(
        &self,
        store: Box<dyn Store + Sync + Send>,
        document_id: String,
        add_tags: Vec<String>,
        remove_tags: Vec<String>,
    ) -> Result<Vec<String>> {
        let qdrant_client = self.qdrant_client().await?;
        let new_tags = store
            .update_data_source_document_tags(
                &self.project,
                &self.data_source_id(),
                &document_id.to_string(),
                &add_tags,
                &remove_tags,
            )
            .await?;
        let mut payload = Payload::new();
        payload.insert("tags", new_tags.clone());
        let field_condition = qdrant::FieldCondition {
            key: "document_id".to_string(),
            r#match: Some(qdrant::Match {
                match_value: Some(qdrant::r#match::MatchValue::Text(document_id)),
            }),
            ..Default::default()
        };
        let points_selector = PointsSelector {
            points_selector_one_of: Some(PointsSelectorOneOf::Filter(Filter {
                must: vec![field_condition.into()],
                ..Default::default()
            })),
        };
        qdrant_client
            .set_payload(
                self.qdrant_collection().to_string(),
                &points_selector,
                payload,
                None,
            )
            .await?;

        Ok(new_tags)
    }

    pub async fn upsert(
        &self,
        credentials: Credentials,
        store: Box<dyn Store + Sync + Send>,
        document_id: &str,
        timestamp: Option<u64>,
        tags: &Vec<String>,
        source_url: &Option<String>,
        text: &str,
        preserve_system_tags: bool,
    ) -> Result<Document> {
        // disallow preserve_system_tags=true if tags contains a string starting with the system tag prefix
        // prevents having duplicate system tags or have users accidentally add system tags (from UI/API)
        if preserve_system_tags
            && tags
                .iter()
                .any(|tag| tag.starts_with(DATA_SOURCE_DOCUMENT_SYSTEM_TAG_PREFIX))
        {
            Err(anyhow!(
                "preserve_system_tags=true is not allowed if `tags` contains a string starting with \"{}\"",
                DATA_SOURCE_DOCUMENT_SYSTEM_TAG_PREFIX
            ))?;
        }

        let store = store.clone();

        let current_system_tags = if preserve_system_tags {
            let current_doc = store
                .load_data_source_document(
                    &self.project,
                    &self.data_source_id(),
                    &document_id.to_string(),
                    &None,
                )
                .await?;

            let current_tags = match current_doc {
                Some(current_doc) => current_doc.tags,
                None => vec![],
            };

            current_tags
                .iter()
                .filter(|tag| tag.starts_with(DATA_SOURCE_DOCUMENT_SYSTEM_TAG_PREFIX))
                .map(|tag| tag.to_string())
                .collect()
        } else {
            vec![]
        };

        let tags: Vec<String> = tags
            .iter()
            .chain(current_system_tags.iter())
            .map(|tag| tag.to_string())
            .collect();

        let timestamp = match timestamp {
            Some(timestamp) => timestamp,
            None => utils::now(),
        };

        // Hash document.
        let mut hasher = blake3::Hasher::new();
        hasher.update(document_id.as_bytes());
        hasher.update(text.as_bytes());
        hasher.update(format!("{}", timestamp).as_bytes());
        tags.iter().for_each(|tag| {
            hasher.update(tag.as_bytes());
        });
        let document_hash = format!("{}", hasher.finalize().to_hex());

        let mut hasher = blake3::Hasher::new();
        hasher.update(document_id.as_bytes());
        let document_id_hash = format!("{}", hasher.finalize().to_hex());

        let mut document = Document::new(
            &self.data_source_id,
            document_id,
            timestamp,
            &tags,
            source_url,
            &document_hash,
            text.len() as u64,
        )?;

        // GCP store raw text and document_id.
        let bucket = match std::env::var("DUST_DATA_SOURCES_BUCKET") {
            Ok(bucket) => bucket,
            Err(_) => Err(anyhow!("DUST_DATA_SOURCES_BUCKET is not set"))?,
        };

        let bucket_path = format!(
            "{}/{}/{}",
            self.project.project_id(),
            self.internal_id,
            document_id_hash
        );

        let document_id_path = format!("{}/document_id.txt", bucket_path);
        let content_path = format!("{}/{}/content.txt", bucket_path, document_hash);
        let tags_path = format!("{}/{}/tags.json", bucket_path, document_hash);
        let timestamp_path = format!("{}/{}/timestamp.txt", bucket_path, document_hash);

        let _ = try_join!(
            Object::create(
                &bucket,
                document_id.as_bytes().to_vec(),
                &document_id_path,
                "application/text",
            ),
            Object::create(
                &bucket,
                text.as_bytes().to_vec(),
                &content_path,
                "application/text",
            ),
            Object::create(
                &bucket,
                serde_json::to_string(&tags).unwrap().as_bytes().to_vec(),
                &tags_path,
                "application/json",
            ),
            Object::create(
                &bucket,
                format!("{}", timestamp).as_bytes().to_vec(),
                &timestamp_path,
                "application/text",
            ),
        )?;

        utils::done(&format!(
            "Created document blob: data_source_id={} document_id={}",
            self.data_source_id, document_id,
        ));

        // Split text in chunks.
        let splits = splitter(self.config.splitter_id)
            .split(
                credentials.clone(),
                self.config.provider_id,
                &self.config.model_id,
                self.config.max_chunk_size,
                text,
            )
            .await?;

        // Embed chunks with max concurrency of 24.
        let e = futures::stream::iter(splits.into_iter().enumerate())
            .map(|(i, s)| {
                let provider_id = self.config.provider_id.clone();
                let model_id = self.config.model_id.clone();
                let credentials = credentials.clone();
                let extras = self.config.extras.clone();
                tokio::spawn(async move {
                    let r = EmbedderRequest::new(provider_id, &model_id, &s, extras);
                    let v = r.execute(credentials).await?;
                    Ok::<(usize, std::string::String, EmbedderVector), anyhow::Error>((i, s, v))
                })
            })
            .buffer_unordered(24)
            .map(|r| match r {
                Err(e) => Err(anyhow!("DataSource chunk embedding error: {}", e))?,
                Ok(r) => r,
            })
            .try_collect::<Vec<_>>()
            .await?;

        utils::done(&format!(
            "Finished embedding chunks: data_source_id={} document_id={} chunk_count={}",
            self.data_source_id,
            document_id,
            e.len(),
        ));

        document.chunks = e
            .into_iter()
            .map(|(i, s, v)| {
                let mut hasher = blake3::Hasher::new();
                hasher.update(document_hash.as_bytes());
                hasher.update(s.as_bytes());
                let hash = format!("{}", hasher.finalize().to_hex());

                Chunk {
                    text: s,
                    hash,
                    offset: i,
                    vector: Some(v.vector),
                    score: None,
                }
            })
            .collect::<Vec<_>>();
        document.chunk_count = document.chunks.len();
        document.token_count = Some(document.chunks.len() * self.config.max_chunk_size);

        // Clean-up previous document chunks (vector search db).
        let qdrant_client = self.qdrant_client().await?;
        let _ = qdrant_client
            .delete_points(
                self.qdrant_collection(),
                &qdrant::Filter {
                    must_not: vec![],
                    should: vec![],
                    must: vec![qdrant::FieldCondition {
                        key: "document_id_hash".to_string(),
                        r#match: Some(qdrant::Match {
                            match_value: Some(qdrant::r#match::MatchValue::Keyword(
                                document_id_hash.clone(),
                            )),
                        }),
                        ..Default::default()
                    }
                    .into()],
                }
                .into(),
                None,
            )
            .await?;

        // Insert new chunks (vector search db).
        let points = document
            .chunks
            .iter()
            .map(|c| {
                let uid = Uuid::new_v4();
                let mut payload = Payload::new();
                payload.insert("tags", document.tags.clone());
                payload.insert("timestamp", document.timestamp as i64);
                payload.insert("chunk_offset", c.offset as i64);
                payload.insert("chunk_hash", c.hash.clone());
                payload.insert("data_source_id", self.data_source_id.clone());
                payload.insert("data_source_internal_id", self.internal_id.clone());
                payload.insert("document_id", document.document_id.clone());
                payload.insert("document_id_hash", document_id_hash.clone());
                payload.insert("text", c.text.clone());

                qdrant::PointStruct::new(
                    uid.to_string(),
                    c.vector
                        .as_ref()
                        .unwrap()
                        .iter()
                        .map(|v| *v as f32)
                        .collect::<Vec<f32>>(),
                    payload,
                )
            })
            .collect::<Vec<_>>();

        if points.len() > 0 {
            let _ = qdrant_client
                .upsert_points(self.qdrant_collection(), points, None)
                .await?;
        }

        utils::done(&format!(
            "Inserted vectors in Qdrant: data_source_id={} document_id={}",
            self.data_source_id, document_id,
        ));

        // Upsert document (SQL)
        store
            .upsert_data_source_document(&self.project, &self.data_source_id, &document)
            .await?;

        Ok(document)
    }

    const MAX_TOP_K_SEARCH: usize = 128;

    pub async fn search(
        &self,
        credentials: Credentials,
        store: Box<dyn Store + Sync + Send>,
        query: &str,
        top_k: usize,
        filter: Option<SearchFilter>,
        full_text: bool,
        target_document_tokens: Option<usize>,
    ) -> Result<Vec<Document>> {
        if top_k > DataSource::MAX_TOP_K_SEARCH {
            return Err(anyhow!("top_k must be <= {}", DataSource::MAX_TOP_K_SEARCH));
        }
        let store = store.clone();

        let r = EmbedderRequest::new(
            self.config.provider_id,
            &self.config.model_id,
            query,
            self.config.extras.clone(),
        );
        let v = r.execute(credentials).await?;

        // Construct the filters for the search query if specified.
        let f = match filter {
            Some(f) => {
                let mut must_filter: Vec<qdrant::Condition> = vec![];
                let mut must_not_filter: Vec<qdrant::Condition> = vec![];

                match f.tags {
                    Some(tags) => {
                        match tags.is_in.clone() {
                            Some(v) => must_filter.push(
                                qdrant::FieldCondition {
                                    key: "tags".to_string(),
                                    r#match: Some(qdrant::Match {
                                        match_value: Some(qdrant::r#match::MatchValue::Keywords(
                                            qdrant::RepeatedStrings { strings: v },
                                        )),
                                    }),
                                    ..Default::default()
                                }
                                .into(),
                            ),
                            None => (),
                        };
                        match tags.is_not.clone() {
                            Some(v) => must_not_filter.push(
                                qdrant::FieldCondition {
                                    key: "tags".to_string(),
                                    r#match: Some(qdrant::Match {
                                        match_value: Some(qdrant::r#match::MatchValue::Keywords(
                                            qdrant::RepeatedStrings { strings: v },
                                        )),
                                    }),
                                    ..Default::default()
                                }
                                .into(),
                            ),
                            None => (),
                        };
                    }
                    None => (),
                };

                match f.timestamp {
                    Some(timestamp) => {
                        match timestamp.gt.clone() {
                            Some(v) => must_filter.push(
                                qdrant::FieldCondition {
                                    key: "timestamp".to_string(),
                                    range: Some(qdrant::Range {
                                        gte: Some(v as f64),
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                }
                                .into(),
                            ),
                            None => (),
                        };
                        match timestamp.lt.clone() {
                            Some(v) => must_filter.push(
                                qdrant::FieldCondition {
                                    key: "timestamp".to_string(),
                                    range: Some(qdrant::Range {
                                        lte: Some(v as f64),
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                }
                                .into(),
                            ),
                            None => (),
                        };
                    }
                    None => (),
                };

                Some(qdrant::Filter {
                    must: must_filter,
                    must_not: must_not_filter,
                    ..Default::default()
                })
            }
            None => None,
        };

        let qdrant_client = self.qdrant_client().await?;
        let results = qdrant_client
            .search_points(&qdrant::SearchPoints {
                collection_name: self.qdrant_collection(),
                vector: v.vector.iter().map(|v| *v as f32).collect::<Vec<f32>>(),
                filter: f,
                limit: top_k as u64,
                with_payload: Some(true.into()),
                params: None,
                score_threshold: None,
                offset: None,
                vector_name: None,
                with_vectors: None,
                read_consistency: None,
            })
            .await?;

        let chunks = results
            .result
            .iter()
            .map(|r| {
                let document_id = match r.payload.get("document_id") {
                    Some(t) => match t.kind {
                        Some(qdrant::value::Kind::StringValue(ref s)) => s.clone(),
                        _ => Err(anyhow!("Missing `document_id` in chunk payload"))?,
                    },
                    None => Err(anyhow!("Missing `document_id` in chunk payload"))?,
                };
                let text = match r.payload.get("text") {
                    Some(t) => match t.kind {
                        Some(qdrant::value::Kind::StringValue(ref s)) => s,
                        _ => Err(anyhow!("Missing `text` in chunk payload"))?,
                    },
                    None => Err(anyhow!("Missing `text` in chunk payload"))?,
                };
                let chunk_hash = match r.payload.get("chunk_hash") {
                    Some(t) => match t.kind {
                        Some(qdrant::value::Kind::StringValue(ref s)) => s,
                        _ => Err(anyhow!("Missing `chunk_hash` in chunk payload"))?,
                    },
                    None => Err(anyhow!("Missing `chunk_hash` in chunk payload"))?,
                };
                let chunk_offset = match r.payload.get("chunk_offset") {
                    Some(t) => match t.kind {
                        Some(qdrant::value::Kind::IntegerValue(i)) => i,
                        _ => Err(anyhow!("Missing `chunk_offset` in chunk payload"))?,
                    },
                    None => Err(anyhow!("Missing `chunk_offset` in chunk payload"))?,
                };
                Ok((
                    document_id,
                    Chunk {
                        text: text.clone(),
                        hash: chunk_hash.clone(),
                        offset: chunk_offset as usize,
                        vector: None,
                        score: Some(r.score as f64),
                    },
                ))
            })
            .collect::<Result<Vec<_>>>()?;

        // get a list of unique document_id
        let document_ids = chunks
            .iter()
            .map(|(document_id, _)| document_id.clone())
            .collect::<std::collections::HashSet<_>>();

        // GCP retrieve raw text and document_id.
        let bucket = match std::env::var("DUST_DATA_SOURCES_BUCKET") {
            Ok(bucket) => bucket,
            Err(_) => Err(anyhow!("DUST_DATA_SOURCES_BUCKET is not set"))?,
        };

        // Retrieve the documents from the store.
        let documents = futures::stream::iter(document_ids)
            .map(|document_id| {
                let store = store.clone();
                let document_id = document_id.clone();
                let data_source_id = self.data_source_id.clone();
                let project = self.project.clone();
                let bucket = bucket.clone();
                let internal_id = self.internal_id.clone();
                tokio::spawn(async move {
                    let mut d: Document = match store
                        .load_data_source_document(&project, &data_source_id, &document_id, &None)
                        .await?
                    {
                        Some(d) => d,
                        None => Err(anyhow!("Document not found"))?,
                    };

                    if full_text {
                        let mut hasher = blake3::Hasher::new();
                        hasher.update(document_id.as_bytes());
                        let document_id_hash = format!("{}", hasher.finalize().to_hex());

                        let bucket_path = format!(
                            "{}/{}/{}",
                            project.project_id(),
                            internal_id,
                            document_id_hash
                        );
                        let content_path = format!("{}/{}/content.txt", bucket_path, d.hash);
                        let bytes = Object::download(&bucket, &content_path).await?;
                        let text = String::from_utf8(bytes)?;

                        d.text = Some(text.clone());
                    }
                    Ok::<Document, anyhow::Error>(d)
                })
            })
            .buffer_unordered(16)
            .map(|r| match r {
                Err(e) => Err(anyhow!("Data source document retrieval error: {}", e))?,
                Ok(r) => r,
            })
            .try_collect::<Vec<_>>()
            .await?;

        // Qdrant client implements the sync and send traits, so we just need
        // to wrap it in an Arc so that it can be cloned.
        let l_qdrant_client = Arc::new(qdrant_client);
        let mut documents = match target_document_tokens {
            Some(target) => {
                futures::stream::iter(documents)
                    .map(|mut d| {
                        let mut chunks = chunks
                            .iter()
                            .filter(|(document_id, _)| document_id == &d.document_id)
                            .map(|(_, c)| c.clone())
                            .collect::<Vec<Chunk>>();
                        let collection = self.qdrant_collection();
                        let chunk_size = self.config.max_chunk_size;
                        let qdrant_client = l_qdrant_client.clone();
                        let mut token_count = chunks.len() * chunk_size;
                        d.token_count = Some(token_count);
                        tokio::spawn(async move {
                            let mut offset_set = std::collections::HashSet::new();
                            for chunk in chunks.iter() {
                                offset_set.insert(chunk.offset);
                            }
                            let current_length = chunks.len() * chunk_size;
                            if (target as i64 - current_length as i64) / chunk_size as i64 <= 0 {
                                d.chunks = chunks;
                                return Ok(d);
                            }
                            let new_offsets = target_document_tokens_offsets(
                                chunks.iter().map(|c| c.offset).collect(),
                                (target - current_length) / chunk_size,
                                d.chunk_count,
                            );
                            let offset_values: Vec<i64> = new_offsets
                                .keys()
                                .cloned()
                                .collect::<Vec<usize>>()
                                .into_iter()
                                .map(|o| o as i64)
                                .collect();
                            let new_offsets_count = offset_values.len() as u32;
                            if new_offsets_count == 0 {
                                d.chunks = chunks;
                                return Ok(d);
                            }

                            let mut hasher = blake3::Hasher::new();
                            hasher.update(d.document_id.as_bytes());
                            let document_id_hash = format!("{}", hasher.finalize().to_hex());
                            let filter = qdrant::Filter {
                                must: vec![
                                    qdrant::FieldCondition {
                                        key: "document_id_hash".to_string(),
                                        r#match: Some(qdrant::Match {
                                            match_value: Some(
                                                qdrant::r#match::MatchValue::Keyword(
                                                    document_id_hash,
                                                ),
                                            ),
                                        }),
                                        ..Default::default()
                                    }
                                    .into(),
                                    qdrant::FieldCondition {
                                        key: "chunk_offset".to_string(),
                                        r#match: Some(qdrant::Match {
                                            match_value: Some(
                                                qdrant::r#match::MatchValue::Integers(
                                                    qdrant::RepeatedIntegers {
                                                        integers: offset_values,
                                                    },
                                                ),
                                            ),
                                        }),
                                        ..Default::default()
                                    }
                                    .into(),
                                ],
                                ..Default::default()
                            };
                            let search_points = qdrant::ScrollPoints {
                                collection_name: collection,
                                filter: Some(filter),
                                limit: Some(new_offsets_count),
                                ..Default::default()
                            };
                            let results_expand = match qdrant_client.scroll(&search_points).await {
                                Ok(r) => r.result,
                                Err(e) => {
                                    utils::error(&format!("Qdrant scroll error: {}", e));
                                    Err(anyhow!("Qdrant scroll error: {}", e))?
                                }
                            };
                            let mut parsed_results = results_expand
                                .iter()
                                .map(|r| {
                                    let text = match r.payload.get("text") {
                                        Some(t) => match t.kind {
                                            Some(qdrant::value::Kind::StringValue(ref s)) => s,
                                            _ => Err(anyhow!("Missing `text` in chunk payload"))?,
                                        },
                                        None => Err(anyhow!("Missing `text` in chunk payload"))?,
                                    };
                                    let chunk_offset = match r.payload.get("chunk_offset") {
                                        Some(t) => match t.kind {
                                            Some(qdrant::value::Kind::IntegerValue(i)) => i,
                                            _ => Err(anyhow!(
                                                "Missing `chunk_offset` in chunk payload"
                                            ))?,
                                        },
                                        None => {
                                            Err(anyhow!("Missing `chunk_offset` in chunk payload"))?
                                        }
                                    };
                                    Ok((text, chunk_offset as usize))
                                })
                                .collect::<Result<Vec<_>>>()?;
                            parsed_results.sort_by(|a, b| a.1.cmp(&b.1));
                            let mut counter = 0;
                            chunks.sort_by(|a, b| a.offset.cmp(&b.offset));
                            chunks = chunks
                                .into_iter()
                                .map(|mut chunk| {
                                    let mut prepend = "".to_owned();
                                    while counter < parsed_results.len()
                                        && *new_offsets.get(&parsed_results[counter].1).unwrap()
                                            == chunk.offset
                                    {
                                        let c_offset = parsed_results[counter].1;
                                        if chunk.offset < c_offset {
                                            chunk.text.push_str(
                                                &(" ".to_owned()
                                                    + &parsed_results[counter].0.clone()),
                                            );
                                        } else {
                                            prepend.push_str(
                                                &(parsed_results[counter].0.clone() + " "),
                                            );
                                        }
                                        counter += 1;
                                        token_count += chunk_size;
                                    }
                                    chunk.text = prepend + &chunk.text;
                                    chunk
                                })
                                .collect::<Vec<_>>();
                            chunks.sort_by(|a, b| {
                                let b_score = b.score.unwrap_or(0.0);
                                let a_score = a.score.unwrap_or(0.0);
                                b_score
                                    .partial_cmp(&a_score)
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            });
                            d.chunks = chunks;
                            d.token_count = Some(token_count);

                            Ok::<Document, anyhow::Error>(d)
                        })
                    })
                    .buffer_unordered(16)
                    .map(|r| match r {
                        Err(e) => Err(anyhow!(
                            "Data source document retrieval expansion error: {}",
                            e
                        ))?,
                        Ok(r) => r,
                    })
                    .try_collect::<Vec<_>>()
                    .await?
            }
            None => documents
                .into_iter()
                .map(|mut d| {
                    let chunks = chunks
                        .iter()
                        .filter(|(document_id, _)| document_id == &d.document_id)
                        .map(|(_, c)| c.clone())
                        .collect::<Vec<Chunk>>();
                    d.token_count = Some(chunks.len() * self.config.max_chunk_size);
                    d.chunks = chunks;
                    d
                })
                .collect::<Vec<_>>(),
        };

        // Sort the documents by the score of the first chunk (guaranteed ordered).
        documents.sort_by(|a, b| {
            let b_score = b.chunks.first().unwrap().score.unwrap_or(0.0);
            let a_score = a.chunks.first().unwrap().score.unwrap_or(0.0);
            b_score
                .partial_cmp(&a_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        utils::done(&format!(
            "Searched Data Source: data_source_id={} document_count={} chunk_count={}",
            self.data_source_id,
            documents.len(),
            documents.iter().map(|d| d.chunks.len()).sum::<usize>(),
        ));

        Ok(documents)
    }

    pub async fn retrieve(
        &self,
        store: Box<dyn Store + Sync + Send>,
        document_id: &str,
        remove_system_tags: bool,
        version_hash: &Option<String>,
    ) -> Result<Option<Document>> {
        let store = store.clone();

        let mut d = match store
            .load_data_source_document(
                &self.project,
                &self.data_source_id,
                document_id,
                version_hash,
            )
            .await?
        {
            Some(d) => d,
            None => {
                return Ok(None);
            }
        };

        d.tags = if remove_system_tags {
            // remove tags that are prefixed with the system tag prefix
            d.tags
                .into_iter()
                .filter(|t| !t.starts_with(DATA_SOURCE_DOCUMENT_SYSTEM_TAG_PREFIX))
                .collect::<Vec<_>>()
        } else {
            d.tags
        };

        let mut hasher = blake3::Hasher::new();
        hasher.update(document_id.as_bytes());
        let document_id_hash = format!("{}", hasher.finalize().to_hex());

        // GCP retrieve raw text and document_id.
        let bucket = match std::env::var("DUST_DATA_SOURCES_BUCKET") {
            Ok(bucket) => bucket,
            Err(_) => Err(anyhow!("DUST_DATA_SOURCES_BUCKET is not set"))?,
        };

        let bucket_path = format!(
            "{}/{}/{}",
            self.project.project_id(),
            self.internal_id,
            document_id_hash
        );
        let content_path = format!("{}/{}/content.txt", bucket_path, d.hash);
        let bytes = Object::download(&bucket, &content_path).await?;
        let text = String::from_utf8(bytes)?;

        d.text = Some(text.clone());

        Ok(Some(d))
    }

    pub async fn delete_document(
        &self,
        store: Box<dyn Store + Sync + Send>,
        document_id: &str,
    ) -> Result<()> {
        let store = store.clone();

        let mut hasher = blake3::Hasher::new();
        hasher.update(document_id.as_bytes());
        let document_id_hash = format!("{}", hasher.finalize().to_hex());

        // Clean-up document chunks (vector search db).
        let qdrant_client = self.qdrant_client().await?;
        let _ = qdrant_client
            .delete_points(
                self.qdrant_collection(),
                &qdrant::Filter {
                    must_not: vec![],
                    should: vec![],
                    must: vec![qdrant::FieldCondition {
                        key: "document_id_hash".to_string(),
                        r#match: Some(qdrant::Match {
                            match_value: Some(qdrant::r#match::MatchValue::Keyword(
                                document_id_hash.clone(),
                            )),
                        }),
                        ..Default::default()
                    }
                    .into()],
                }
                .into(),
                None,
            )
            .await?;

        // Delete document (SQL)
        store
            .delete_data_source_document(&self.project, &self.data_source_id, document_id)
            .await?;

        Ok(())
    }

    pub async fn delete(&self, store: Box<dyn Store + Sync + Send>) -> Result<()> {
        let store = store.clone();

        // Delete collection (vector search db).
        let qdrant_client = self.qdrant_client().await?;
        qdrant_client
            .delete_collection(self.qdrant_collection())
            .await?;

        utils::done(&format!(
            "Deleted QDrant collection: data_source_id={}",
            self.data_source_id,
        ));

        // Delete Data Source and documents (SQL)
        store
            .delete_data_source(&self.project, &self.data_source_id)
            .await?;

        utils::done(&format!(
            "Deleted Data Source records: data_source_id={}",
            self.data_source_id,
        ));

        Ok(())
    }
}

pub async fn cmd_register(data_source_id: &str, config: &DataSourceConfig) -> Result<()> {
    let root_path = utils::init_check().await?;
    let store = SQLiteStore::new(root_path.join("store.sqlite"))?;
    store.init().await?;
    let project = Project::new_from_id(1);

    let ds = DataSource::new(&project, data_source_id, config);

    ds.setup(Credentials::new()).await?;
    store.register_data_source(&project, &ds).await?;

    utils::done(&format!("Registered data_source `{}`", ds.data_source_id(),));

    Ok(())
}

pub async fn cmd_upsert(
    data_source_id: &str,
    document_id: &str,
    timestamp: Option<u64>,
    tags: &Vec<String>,
    source_url: &Option<String>,
    text_path: &str,
) -> Result<()> {
    let root_path = utils::init_check().await?;
    let store = SQLiteStore::new(root_path.join("store.sqlite"))?;
    store.init().await?;
    let project = Project::new_from_id(1);

    let ds = match store.load_data_source(&project, data_source_id).await? {
        Some(ds) => ds,
        None => Err(anyhow!("Data source `{}` not found", data_source_id))?,
    };

    let text_path = &shellexpand::tilde(text_path).into_owned();
    let text_path = std::path::Path::new(text_path);

    let contents = async_fs::read(text_path).await?;
    let text = std::str::from_utf8(&contents)?;

    let d = ds
        .upsert(
            Credentials::new(),
            Box::new(store.clone()),
            document_id,
            timestamp,
            tags,
            source_url,
            text,
            true, // preserve system tags
        )
        .await?;

    utils::done(&format!(
        "Upserted document: data_source={} document_id={} text_length={} chunk_count={} tags={}",
        ds.data_source_id(),
        document_id,
        text.len(),
        d.chunks.len(),
        tags.join(","),
    ));

    Ok(())
}

pub async fn cmd_search(data_source_id: &str, query: &str, top_k: usize) -> Result<()> {
    let root_path = utils::init_check().await?;
    let store = SQLiteStore::new(root_path.join("store.sqlite"))?;
    store.init().await?;
    let project = Project::new_from_id(1);

    let ds = match store.load_data_source(&project, data_source_id).await? {
        Some(ds) => ds,
        None => Err(anyhow!("Data source `{}` not found", data_source_id))?,
    };

    let r = ds
        .search(
            Credentials::new(),
            Box::new(store.clone()),
            query,
            top_k,
            None,
            false,
            None,
        )
        .await?;

    utils::info(&format!(
        "{} documents, {} chunks total",
        r.len(),
        r.iter().map(|d| d.chunks.len()).sum::<usize>(),
    ));
    r.iter().for_each(|d| {
        utils::info(&format!(
            "- Document: document_id={} text_size={} chunk_count={}",
            d.document_id, d.text_size, d.chunk_count,
        ));
        d.chunks.iter().for_each(|c| {
            utils::info(&format!(
                "  > Chunk: offset={} score={}",
                c.offset,
                c.score.unwrap_or(0.0),
            ));
            println!("```\n{}\n```", c.text);
        });
    });

    Ok(())
}

pub async fn cmd_retrieve(data_source_id: &str, document_id: &str) -> Result<()> {
    let root_path = utils::init_check().await?;
    let store = SQLiteStore::new(root_path.join("store.sqlite"))?;
    store.init().await?;
    let project = Project::new_from_id(1);

    let ds = match store.load_data_source(&project, data_source_id).await? {
        Some(ds) => ds,
        None => Err(anyhow!("Data source `{}` not found", data_source_id))?,
    };

    let d = match ds
        .retrieve(Box::new(store.clone()), document_id, true, &None)
        .await?
    {
        Some(d) => d,
        None => Err(anyhow!("Document not found: document_id={}", document_id))?,
    };

    utils::done(&format!(
        "Retrieved document: data_source={} document_id={}",
        ds.data_source_id(),
        document_id,
    ));

    utils::info(&format!(
        "- Document: document_id={} text_size={} chunk_count={}",
        d.document_id, d.text_size, d.chunk_count,
    ));

    match d.text {
        Some(text) => {
            println!("```\n{}\n```", text);
        }
        None => (),
    }

    Ok(())
}

pub async fn cmd_delete(data_source_id: &str, document_id: &str) -> Result<()> {
    let root_path = utils::init_check().await?;
    let store = SQLiteStore::new(root_path.join("store.sqlite"))?;
    store.init().await?;
    let project = Project::new_from_id(1);

    let ds = match store.load_data_source(&project, data_source_id).await? {
        Some(ds) => ds,
        None => Err(anyhow!("Data source `{}` not found", data_source_id))?,
    };

    ds.delete_document(Box::new(store.clone()), document_id)
        .await?;

    utils::done(&format!(
        "Deleted document: data_source={} document_id={}",
        ds.data_source_id(),
        document_id,
    ));

    Ok(())
}

pub async fn cmd_list(data_source_id: &str) -> Result<()> {
    let root_path = utils::init_check().await?;
    let store = SQLiteStore::new(root_path.join("store.sqlite"))?;
    store.init().await?;
    let project = Project::new_from_id(1);

    let r = store
        .list_data_source_documents(
            &project,
            data_source_id,
            None,
            true, // remove system tags
        )
        .await?;

    utils::info(&format!("{} documents", r.0.len(),));
    r.0.iter().for_each(|d| {
        utils::info(&format!(
            "- Document: document_id={} text_size={} chunk_count={}",
            d.document_id, d.text_size, d.chunk_count,
        ));
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_c() {
        let tests = HashMap::from([
            (
                (vec![1, 4, 5], 6, 8),
                HashMap::from([(0, 1), (2, 1), (3, 4), (6, 5), (7, 5)]),
            ),
            (
                (vec![7, 9, 11], 18, 18),
                HashMap::from([
                    (2, 7),
                    (3, 7),
                    (4, 7),
                    (5, 7),
                    (6, 7),
                    (8, 7),
                    (10, 9),
                    (12, 11),
                    (13, 11),
                    (14, 11),
                    (15, 11),
                    (16, 11),
                    (17, 11),
                ]),
            ),
            (
                (vec![0, 31], 6, 32),
                HashMap::from([(1, 0), (2, 0), (3, 0), (28, 31), (29, 31), (30, 31)]),
            ),
            ((vec![0, 1], 6, 32), HashMap::from([(2, 1), (3, 1), (4, 1)])),
            (
                (vec![0, 2], 6, 32),
                HashMap::from([(3, 2), (4, 2), (5, 2), (1, 0)]),
            ),
            (
                (vec![30, 31], 6, 32),
                HashMap::from([(27, 30), (28, 30), (29, 30)]),
            ),
            ((vec![29, 31], 6, 32), HashMap::from([(28, 29), (30, 29)])),
            (
                (vec![15, 16], 6, 32),
                HashMap::from([(12, 15), (13, 15), (14, 15), (17, 16), (18, 16), (19, 16)]),
            ),
            (
                (vec![4, 20], 6, 32),
                HashMap::from([(3, 4), (5, 4), (19, 20), (21, 20)]),
            ),
        ]);
        // execute every test:
        for ((offsets, text_size, chunk_size), result) in tests {
            assert_eq!(
                target_document_tokens_offsets(offsets, text_size, chunk_size),
                result
            );
        }
    }
}
