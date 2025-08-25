Title: Vector Store — Qdrant

Scope
- Store, update, delete, and search semantic vectors for data source content with strict tenant separation.

Key Code
- Client management & config: `core/src/data_sources/qdrant.rs`.
- Cluster enum: `QdrantCluster` with env-var prefixes (e.g., `QDRANT_CLUSTER_0`).

Behavior
- Collection naming ties to embedder provider+model (e.g., `c_openai_text-embedding-3-large-1536`).
- Sharding: shard key derived from data source internal id (first 16 hex chars -> modulo `SHARD_KEY_COUNT`).
- Tenant isolation: every filter augmented with `data_source_internal_id` equality; enforced in all ops.

Operations
- `delete_all_points_for_internal_id`: wipe all points for a data source.
- `delete_points`, `scroll`, `search` (see file for variants) scoped by shard key and tenant filter.

Environment
- `QDRANT_CLUSTER_0_URL`, `QDRANT_CLUSTER_0_API_KEY` (and similarly for any additional clusters if added).

Extensibility
- Add clusters: extend `QDRANT_CLUSTER_VARIANTS` and env var prefix logic.
- Shadow-write migrations: collection naming includes provider/model to support dual-writes.

