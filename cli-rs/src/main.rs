use anyhow::{anyhow, bail, Context, Result};
use clap::{ArgGroup, Args, Parser, Subcommand};
use dust::stores::store::Store;
use std::fs;
mod block;
mod block_validation;
mod docs;
mod pg;
mod util;

/// dustx: Minimal CLI for Dust Core (local-first)
#[derive(Parser, Debug)]
#[command(name = "dustx", version, about = "Dust CLI (local-first)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug, Clone)]
enum SpecCmd {
    /// Parse and validate a .dust specification, printing its hash and block list
    Check {
        #[arg(value_name = "SPEC_PATH")]
        spec_path: String,
        #[arg(long)]
        json: bool,
    },
    /// Format a .dust spec file (canonical ordering/spacing)
    Fmt {
        #[arg(value_name = "SPEC_PATH")]
        spec_path: String,
    },
    /// Compute the spec hash
    Hash {
        #[arg(value_name = "SPEC_PATH")]
        spec_path: String,
    },
    /// Show unified diff; default compares formatted vs current. Optionally against a file.
    Diff {
        #[arg(value_name = "SPEC_PATH")]
        spec_path: String,
        #[arg(long, value_name = "PATH")]
        against: Option<String>,
    },
    /// Estimate token usage for prompts/instructions
    Tokens {
        #[arg(value_name = "SPEC_PATH")]
        spec_path: String,
        #[arg(long, value_name = "MODEL", default_value = "openai/gpt-4o")]
        model: String,
    },
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Grouped spec commands
    Spec {
        #[command(subcommand)]
        cmd: SpecCmd,
    },
    /// Environment and runtime checks (embedded PG, providers, core)
    Doctor,
    /// List available models for configured providers
    Models,
    /// Execute a .dust spec or an assistant config
    Run(RunArgs),
    /// Authentication and credentials commands
    Auth {
        #[command(subcommand)]
        cmd: AuthCmd,
    },
    /// Cache operations (stubs)
    Cache {
        #[command(subcommand)]
        cmd: CacheCmd,
    },
    /// Block manipulation operations
    Block {
        #[command(subcommand)]
        cmd: BlockCmd,
    },
    /// Watch inputs (spec or assistant file + messages) and re-run on changes
    Dev {
        #[arg(value_name = "SPEC_PATH")]
        spec_path: Option<String>,
        #[arg(long)]
        assistant_file: Option<String>,
        #[arg(long)]
        messages_file: Option<String>,
        #[arg(long)]
        json: bool,
    },
    // /// Embedded Postgres lifecycle
    Db {
        #[command(subcommand)]
        cmd: DbCmd,
    },
    /// Local docs indexing/search (early RAG helper)
    Docs {
        #[command(subcommand)]
        cmd: DocsCmd,
    },
    /// Dataset operations
    Dataset {
        #[command(subcommand)]
        cmd: DatasetCmd,
    },
}

#[derive(Subcommand, Debug, Clone)]
enum CacheCmd {
    Stats,
    Clear {
        #[arg(long)]
        provider: Option<String>,
        #[arg(long, value_name = "YYYY-MM-DD")]
        before: Option<String>,
    },
    Export {
        #[arg(value_name = "FILE")]
        file: String,
        #[arg(long, value_name = "YYYY-MM-DD")]
        before: Option<String>,
        #[arg(long, value_name = "TYPE")]
        r#type: Option<String>,
    },
    Import {
        #[arg(value_name = "FILE")]
        file: String,
    },
}

#[derive(Subcommand, Debug, Clone)]
enum AuthCmd {
    /// Show provider auth status (no secrets)
    Status,
    /// List locally stored auth keys (plaintext)
    List,
    /// Generate a .env.example file
    EnvInit,
    /// Set local credentials (plaintext under .dust/credentials.json)
    Set {
        #[arg(long = "env", value_name = "KEY=VALUE")]
        pairs: Vec<String>,
    },
}

#[derive(Subcommand, Debug, Clone)]
enum BlockCmd {
    /// Append a block scaffold to a spec file
    Add {
        #[arg(value_name = "SPEC_PATH")]
        spec_path: String,
        #[arg(value_name = "TYPE")]
        block_type: String,
        #[arg(value_name = "NAME")]
        name: String,
    },
    /// Remove a block by name
    Rm {
        #[arg(value_name = "SPEC_PATH")]
        spec_path: String,
        #[arg(value_name = "NAME")]
        name: String,
    },
    /// Rename a block
    Rename {
        #[arg(value_name = "SPEC_PATH")]
        spec_path: String,
        #[arg(value_name = "OLD")]
        old: String,
        #[arg(value_name = "NEW")]
        new: String,
    },
    /// Move a block before/after another block by name
    Move {
        #[arg(value_name = "SPEC_PATH")]
        spec_path: String,
        #[arg(value_name = "NAME")]
        name: String,
        #[arg(long)]
        before: Option<String>,
        #[arg(long)]
        after: Option<String>,
    },
    /// Set config key=value pairs on a block (raw values)
    SetConfig {
        #[arg(value_name = "SPEC_PATH")]
        spec_path: String,
        #[arg(value_name = "NAME")]
        name: String,
        #[arg(value_name = "PAIRS")]
        kv: Vec<String>,
    },
}

#[derive(Subcommand, Debug, Clone)]
enum DbCmd {
    /// Start embedded Postgres (writes DSN to CORE_DATABASE_URI)
    Start {
        #[arg(long, value_name = "PATH")]
        data_dir: Option<String>,
        #[arg(long, value_name = "PORT")]
        port: Option<u16>,
    },
    /// Show embedded Postgres status
    Status,
    /// Stop embedded Postgres
    Stop,
    /// Backup the data dir into a zip archive
    Backup {
        #[arg(long, value_name = "ZIP")]
        out: String,
    },
    /// Reset the data dir (optionally keep a backup copy)
    Reset {
        #[arg(long)]
        keep_backup: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
enum DocsCmd {
    /// Index a file or folder into .dust/local_docs
    Index {
        #[arg(value_name = "PATH")]
        path: String,
        #[arg(long)]
        incremental: bool,
    },
    /// Search the local index
    Search {
        #[arg(value_name = "QUERY")]
        query: String,
        #[arg(long, default_value_t = 5)]
        k: usize,
    },
    /// Remove the index
    Clear,
    /// Show index stats
    Stats,
}

#[derive(Subcommand, Debug, Clone)]
enum DatasetCmd {
    /// Create a dataset from a JSONL or JSON file
    Create {
        #[arg(value_name = "DATASET_ID")]
        id: String,
        #[arg(value_name = "FILE")]
        file: String,
    },
    /// List datasets and available hashes
    List,
    /// Show hashes for a dataset id
    Hashes {
        #[arg(value_name = "DATASET_ID")]
        id: String,
    },
    /// Show basic info for a dataset
    Show {
        #[arg(value_name = "DATASET_ID")]
        id: String,
        #[arg(value_name = "HASH")]
        hash: String,
    },
}

#[derive(Args, Debug, Clone)]
#[command(group(ArgGroup::new("runner").required(true).args(["spec_path","assistant_file"])))]
struct RunArgs {
    /// Path to the .dust spec file
    #[arg(value_name = "SPEC_PATH")]
    spec_path: Option<String>,
    /// Assistant configuration file (JSON); generates a chat spec on-the-fly
    #[arg(long, value_name = "FILE")]
    assistant_file: Option<String>,
    /// Optional JSON file with messages or { messages: [...] }
    #[arg(long, value_name = "PATH")]
    messages_file: Option<String>,
    /// Load dataset by id/hash for Input block
    #[arg(long, value_name = "ID")]
    dataset_id: Option<String>,
    /// Load dataset by id/hash for Input block
    #[arg(long, value_name = "HASH")]
    dataset_hash: Option<String>,
    /// Print structured JSON output
    #[arg(long)]
    json: bool,
    /// Disable all provider/cache usage even if blocks enable it
    #[arg(long)]
    no_cache: bool,
    /// Hard cap on estimated total tokens (prompts + instructions + messages)
    #[arg(long, value_name = "N")]
    max_total_tokens: Option<usize>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env if present, fallback to .env.local (non-fatal)
    if dotenvy::from_filename(".env").is_err() {
        let _ = dotenvy::from_filename(".env.local");
    }
    // Initialize JS executor (safe to call multiple times)
    dust::deno::js_executor::JSExecutor::init();
    util::load_local_credentials();
    let cli = Cli::parse();
    let res = match &cli.command {
        Commands::Spec { cmd } => match cmd.clone() {
            SpecCmd::Check { spec_path, json } => spec_check(spec_path.as_str(), json).await,
            SpecCmd::Fmt { spec_path } => spec_fmt(spec_path.as_str()).await,
            SpecCmd::Hash { spec_path } => spec_hash(spec_path.as_str()).await,
            SpecCmd::Diff { spec_path, against } => {
                spec_diff(spec_path.as_str(), against.as_deref()).await
            }
            SpecCmd::Tokens { spec_path, model } => {
                spec_tokens(spec_path.as_str(), model.as_str()).await
            }
        },
        Commands::Doctor => doctor().await,
        Commands::Models => list_models().await,
        Commands::Run(args) => run_once(args.clone()).await,
        Commands::Auth { cmd } => auth_cmd(cmd.clone()).await,
        Commands::Cache { cmd } => cache_cmd(cmd.clone()).await,
        Commands::Block { cmd } => block_cmd(cmd.clone()).await,
        Commands::Dev {
            spec_path,
            assistant_file,
            messages_file,
            json,
        } => {
            dev_watch(RunArgs {
                spec_path: spec_path.clone(),
                assistant_file: assistant_file.clone(),
                messages_file: messages_file.clone(),
                dataset_id: None,
                dataset_hash: None,
                json: *json,
                no_cache: false,
                max_total_tokens: None,
            })
            .await
        }
        Commands::Db { cmd } => db_cmd(cmd.clone()).await,
        Commands::Docs { cmd } => docs_cmd(cmd.clone()).await,
        Commands::Dataset { cmd } => dataset_cmd(cmd.clone()).await,
    };
    if let Err(e) = res {
        use std::process::exit;
        // Show the full error chain with all context
        eprintln!("error: {:#}", e);
        let code = match &cli.command {
            Commands::Spec { .. } => 1,
            Commands::Run(_) => 2,
            Commands::Db { .. } => 4,
            Commands::Doctor => 5,
            _ => 1,
        };
        exit(code);
    }
    Ok(())
}

async fn spec_check(path: &str, json: bool) -> Result<()> {
    let spec = fs::read_to_string(path).with_context(|| {
        format!(
            "Failed to read spec file '{}': not found or unreadable",
            path
        )
    })?;

    // Parse using Core library. This does not execute the spec or require DB.
    let app = match dust::app::App::new(&spec).await {
        Ok(app) => app,
        Err(e) => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "ok": false,
                        "error": format!("{:#}", e)
                    })
                );
                return Ok(()); // JSON mode handles its own output
            } else {
                // Return the error with context - main handler will display it
                return Err(e).with_context(|| format!("Spec validation failed for '{}'", path));
            }
        }
    };

    let hash = app.hash().to_string();
    let blks = app.blocks();

    // Disallow unsupported block types (DataSource/Database) in this release
    let locs = util::approximate_block_lines(&spec);
    let mut unsupported: Vec<(String, usize)> = Vec::new();
    for (bt, name) in &blks {
        use dust::blocks::block::BlockType::*;
        if matches!(bt, DataSource | Database | DatabaseSchema) {
            unsupported.push((bt.to_string(), locs.get(name).cloned().unwrap_or(0)));
        }
    }

    if !unsupported.is_empty() {
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "ok": false,
                    "error": "unsupported block types for this release",
                    "unsupported": unsupported
                })
            );
        } else {
            eprintln!("Unsupported block types in {}:", path);
            for (t, ln) in &unsupported {
                eprintln!(" - {} (line {})", t, ln);
            }
        }
        bail!("spec contains unsupported blocks")
    }

    // Collect block list (type, name) for output
    let blocks: Vec<(String, String)> = blks.into_iter().map(|(t, n)| (t.to_string(), n)).collect();

    if json {
        let out = serde_json::json!({
            "hash": hash,
            "blocks": blocks,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("OK {}", path);
        println!("hash: {}", hash);
        if blocks.is_empty() {
            println!("blocks: (none)");
        } else {
            println!("blocks:");
            for (t, n) in blocks {
                println!("  - {} {}", t, n);
            }
        }
    }
    Ok(())
}

use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

async fn run_once(args: RunArgs) -> Result<()> {
    let mut messages_value: Option<Value> = None;
    if let Some(path) = args.messages_file.as_deref() {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("Failed to read messages file '{}'", path))?;
        messages_value =
            Some(serde_json::from_str::<Value>(&raw).context("messages file is not valid JSON")?);
    }

    let spec = if let Some(spec_path) = &args.spec_path {
        fs::read_to_string(spec_path).with_context(|| {
            format!(
                "Failed to read spec file '{}': not found or unreadable",
                spec_path
            )
        })?
    } else if let Some(assistant_file) = &args.assistant_file {
        let cfg_raw = fs::read_to_string(assistant_file)
            .with_context(|| format!("Failed to read assistant file '{}'", assistant_file))?;
        let cfg: Value =
            serde_json::from_str(&cfg_raw).context("assistant file is not valid JSON")?;
        // Build optional context using local docs based on last user message
        let context = if let Some(msgs) = messages_value.as_ref() {
            let ctx = maybe_local_docs_context(&cfg, msgs).ok().flatten();
            if let Some(ref c) = ctx {
                util::emit_event(
                    "orchestrator_summary",
                    serde_json::json!({"window_chars": c.len()}),
                );
            }
            ctx
        } else {
            None
        };
        build_chat_spec_from_assistant(&cfg, context)
    } else {
        bail!("either SPEC_PATH or --assistant-file is required")
    };

    let mut app = dust::app::App::new(&spec)
        .await
        .with_context(|| "Spec parse failed: invalid Dust syntax")?;

    // Disallow unsupported block types in this release (explicit guard)
    if app.blocks().iter().any(|(bt, _)| {
        matches!(
            bt,
            dust::blocks::block::BlockType::DataSource
                | dust::blocks::block::BlockType::Database
                | dust::blocks::block::BlockType::DatabaseSchema
        )
    }) {
        anyhow::bail!("This spec contains DataSource/Database blocks which are not supported in this release. Use local_docs for RAG or remove these blocks.");
    }

    // Ensure embedded PG is running and DSN set
    let (store, project) = get_store_and_project().await?;

    // Preflight: does the spec require a dataset (Input block present)?
    let requires_dataset = app
        .blocks()
        .iter()
        .any(|(bt, _)| matches!(bt, dust::blocks::block::BlockType::Input));

    let dataset = if let Some(v) = messages_value.clone() {
        let record = match v {
            Value::Array(_) => Value::Object([("messages".to_string(), v)].into_iter().collect()),
            Value::Object(o) => Value::Object(o),
            _ => bail!("messages file must be an array or an object"),
        };
        Some(
            dust::dataset::Dataset::new_from_jsonl("INPUT", vec![record])
                .await
                .context("failed to build input dataset")?,
        )
    } else if args.dataset_id.is_some() || args.dataset_hash.is_some() {
        let id = args
            .dataset_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("--dataset-id required with --dataset-hash"))?;
        let hash = args
            .dataset_hash
            .clone()
            .ok_or_else(|| anyhow::anyhow!("--dataset-hash required with --dataset-id"))?;
        let ds = dust::dataset::Dataset::from_hash(&store, &project, &id, &hash)
            .await?
            .ok_or_else(|| anyhow::anyhow!("dataset '{}' with hash '{}' not found", id, hash))?;
        Some(ds)
    } else {
        None
    };

    if requires_dataset && dataset.is_none() {
        bail!("This spec has an Input block and requires a dataset. Provide --messages-file, or use --dataset-id and --dataset-hash.");
    }

    // Build run config, applying global cache policy if requested
    let mut blocks_cfg: HashMap<String, Value> = HashMap::new();
    if args.no_cache {
        for (bt, name) in app.blocks() {
            use dust::blocks::block::BlockType::*;
            if matches!(bt, LLM | Chat | Curl | Browser | Search) {
                blocks_cfg.insert(name.clone(), serde_json::json!({"use_cache": false}));
            }
        }
    }
    let run_config = dust::run::RunConfig { blocks: blocks_cfg };

    // Optional: estimate tokens and enforce max_total_tokens if requested
    if let Some(cap) = args.max_total_tokens {
        let model_hint =
            std::env::var("DUST_MODEL_HINT").unwrap_or_else(|_| "openai/gpt-4o".to_string());
        let est = estimate_total_tokens(&spec, messages_value.as_ref(), &model_hint)
            .await
            .unwrap_or(0);
        util::emit_event(
            "token_estimate",
            serde_json::json!({"estimated_total_tokens": est, "model": model_hint}),
        );
        if let Ok(rate_str) = std::env::var("DUST_COST_PER_1K_TOKENS_USD") {
            if let Ok(rate) = rate_str.parse::<f64>() {
                let usd = (est as f64) * rate / 1000.0;
                util::emit_event(
                    "cost_estimate",
                    serde_json::json!({"estimated_cost_usd": usd}),
                );
            }
        }
        if est > cap {
            anyhow::bail!(format!(
                "estimated tokens {} exceed cap {} (use --max-total-tokens to raise)",
                est, cap
            ));
        }
    }
    let mut credentials: HashMap<String, String> = HashMap::new();
    for k in [
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "SERP_API_KEY",
        "BROWSERLESS_API_KEY",
    ] {
        if let Ok(v) = std::env::var(k) {
            credentials.insert(k.to_string(), v);
        }
    }
    credentials.insert("DUST_IS_SYSTEM_RUN".to_string(), "true".to_string());
    let secrets = dust::run::Secrets {
        redacted: true,
        secrets: HashMap::new(),
    };

    app.prepare_run(
        dust::run::RunType::Deploy,
        run_config,
        project.clone(),
        dataset,
        Box::new(store.clone()),
    )
    .await
    .context("prepare_run failed")?;

    let databases_store: Box<dyn dust::databases_store::store::DatabasesStore + Sync + Send> =
        Box::new(dust::databases_store::gcs::GoogleCloudStorageDatabasesStore::new());
    util::set_env_default("QDRANT_CLUSTER_0_URL", "http://localhost:6334");
    util::set_env_default("QDRANT_CLUSTER_0_API_KEY", "dev");
    let qdrant_clients = dust::data_sources::qdrant::QdrantClients::build()
        .await
        .context("failed to construct qdrant clients (set QDRANT envs)")?;

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<serde_json::Value>();
    let run_fut = app.run(
        credentials,
        secrets,
        Box::new(store.clone()),
        databases_store,
        qdrant_clients,
        Some(tx),
        true,
    );

    let printer = tokio::spawn(async move {
        while let Some(ev) = rx.recv().await {
            println!("{}", ev);
        }
    });
    let res = run_fut.await;
    let _ = printer.await;
    res.context("spec run failed")
}

async fn doctor() -> Result<()> {
    use std::env;
    println!("dustx doctor");

    // Check embedded PG DSN env (CORE_DATABASE_URI)
    let db_uri = env::var("CORE_DATABASE_URI").unwrap_or_default();
    if db_uri.is_empty() {
        eprintln!("[warn] CORE_DATABASE_URI not set; runs will auto-manage embedded PG but you may want to set a DSN pointing to .dust/db");
    } else {
        println!("[ok] CORE_DATABASE_URI set");
    }

    // Check provider envs (non-fatal)
    let providers = [
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "SERP_API_KEY",
        "BROWSERLESS_API_KEY",
    ];
    let mut any_provider = false;
    for k in providers {
        if env::var(k).is_ok() {
            any_provider = true;
            println!("[ok] provider {} configured", k);
        }
    }
    if !any_provider {
        eprintln!("[warn] no provider credentials found; LLM/web tools may fail. Use 'dust auth set' or export env vars.");
    }

    // Try parsing a tiny no-op spec to validate parser linkage
    let demo =
        "\ninput INPUT {}\ncode CHECK {\n  code:\n```\n_fun = (env) => ({ ok: true })\n```\n}\n";
    let _ = dust::app::App::new(demo)
        .await
        .context("core parse check failed")?;
    println!("[ok] core parse path working");

    println!("[info] {}", pg::status());

    // Ensure embedded PG is running and try a store roundtrip
    let mut hard_error = false;
    match pg::ensure_running().await {
        Ok((dsn, _pg)) => {
            // _pg keeps the PostgreSQL instance alive for this scope
            match dust::stores::postgres::PostgresStore::new(&dsn).await {
                Ok(store) => {
                    if let Err(e) = store.init().await {
                        eprintln!("[warn] init schema failed: {}", e);
                        hard_error = true;
                    }
                    match load_or_create_project(&store).await {
                        Ok(_proj) => println!("[ok] embedded Postgres reachable at {}", dsn),
                        Err(e) => {
                            eprintln!("[warn] store roundtrip failed: {}", e);
                            hard_error = true;
                        }
                    }
                    // Try DB version
                    if let Ok((client, connection)) =
                        tokio_postgres::connect(&dsn, tokio_postgres::NoTls).await
                    {
                        // Spawn the connection pump
                        tokio::spawn(async move {
                            let _ = connection.await;
                        });
                        if let Ok(row) = client.query_one("SELECT version()", &[]).await {
                            let v: &str = row.get(0);
                            println!("[ok] postgres version: {}", v);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[warn] could not connect Postgres store: {}", e);
                    hard_error = true;
                }
            }
        }
        Err(e) => {
            eprintln!("[warn] could not ensure embedded PG: {}", e);
            hard_error = true;
        }
    }

    // JS executor probe
    println!("[ok] JS executor initialized");

    // Filesystem permissions: ensure .dust is writable
    match pg::cli_data_dir() {
        Ok(dir) => {
            let test = dir.join(".doctor_write_test");
            match std::fs::write(&test, b"ok") {
                Ok(_) => {
                    let _ = std::fs::remove_file(&test);
                    println!("[ok] .dust writable at {}", dir.display());
                }
                Err(e) => {
                    eprintln!("[warn] cannot write to {}: {}", dir.display(), e);
                    hard_error = true;
                }
            }
        }
        Err(e) => {
            eprintln!("[warn] cannot resolve data dir: {}", e);
            eprintln!("[hint] check current working directory and permissions");
            hard_error = true;
        }
    }

    // Provider probes (best-effort)
    if let Ok(k) = std::env::var("OPENAI_API_KEY") {
        if !k.is_empty() {
            if let Err(e) = probe_openai(&k).await {
                eprintln!("[warn] openai probe failed: {}", e);
            } else {
                println!("[ok] openai reachable");
            }
        }
    }
    if let Ok(k) = std::env::var("ANTHROPIC_API_KEY") {
        if !k.is_empty() {
            if let Err(e) = probe_anthropic(&k).await {
                eprintln!("[warn] anthropic probe failed: {}", e);
            } else {
                println!("[ok] anthropic reachable");
            }
        }
    }

    println!("doctor completed");
    if hard_error {
        anyhow::bail!("doctor detected blocking issues")
    } else {
        Ok(())
    }
}

async fn get_store_and_project() -> Result<(
    dust::stores::postgres::PostgresStore,
    dust::project::Project,
)> {
    let (db_uri, _) = pg::ensure_running().await?;
    // Ensure plpgsql is available (some installs miss it by default)
    if let Ok((client, connection)) = tokio_postgres::connect(&db_uri, tokio_postgres::NoTls).await
    {
        tokio::spawn(async move {
            let _ = connection.await;
        });
        let _ = client
            .execute("CREATE EXTENSION IF NOT EXISTS plpgsql", &[])
            .await;
    }
    let store = dust::stores::postgres::PostgresStore::new(&db_uri)
        .await
        .context("failed to connect Postgres store")?;
    // Ensure tables, indexes and functions exist (first-run)
    if let Err(e) = store.init().await {
        // Retry once after ensuring plpgsql
        if let Ok((client, connection)) =
            tokio_postgres::connect(&db_uri, tokio_postgres::NoTls).await
        {
            tokio::spawn(async move {
                let _ = connection.await;
            });
            let _ = client
                .execute("CREATE EXTENSION IF NOT EXISTS plpgsql", &[])
                .await;
        }
        store
            .init()
            .await
            .context(format!("failed to initialize database schema: {}", e))?;
    }
    let project = load_or_create_project(&store).await?;
    Ok((store, project))
}

async fn load_or_create_project(
    store: &dust::stores::postgres::PostgresStore,
) -> Result<dust::project::Project> {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct ProjectFile {
        project_id: i64,
    }
    let path = pg::cli_data_dir()?.join("project.json");
    if let Ok(bytes) = std::fs::read(&path) {
        if let Ok(pf) = serde_json::from_slice::<ProjectFile>(&bytes) {
            return Ok(dust::project::Project::new_from_id(pf.project_id));
        }
    }
    let project = store.create_project().await.context("create project")?;
    let pf = ProjectFile {
        project_id: project.project_id(),
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&path, serde_json::to_vec_pretty(&pf)?).ok();
    Ok(project)
}

async fn dataset_create(id: &str, file: &str) -> Result<()> {
    let (store, project) = get_store_and_project().await?;
    let path = Path::new(file);
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", file))?;
    let values: Vec<Value> = if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
        text.lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str::<Value>(l).context("parse jsonl line"))
            .collect::<Result<Vec<_>>>()?
    } else {
        let v: Value = serde_json::from_str(&text).context("parse json")?;
        match v {
            Value::Array(arr) => arr,
            Value::Object(_) => vec![v],
            _ => anyhow::bail!("dataset JSON must be an array of objects or a single object"),
        }
    };
    let ds = dust::dataset::Dataset::new_from_jsonl(id, values).await?;
    let hash = ds.hash().to_string();
    store
        .register_dataset(&project, &ds)
        .await
        .context("register dataset")?;
    println!("{}", hash);
    Ok(())
}

async fn dataset_list() -> Result<()> {
    let (store, project) = get_store_and_project().await?;
    let map = store.list_datasets(&project).await?;
    for (id, entries) in map.iter() {
        println!("{} ({} versions)", id, entries.len());
    }
    Ok(())
}

async fn dataset_hashes(id: &str) -> Result<()> {
    let (store, project) = get_store_and_project().await?;
    let map = store.list_datasets(&project).await?;
    if let Some(entries) = map.get(id) {
        // entries: Vec<(hash, created)>
        for (h, created) in entries {
            println!("{} {}", created, h);
        }
    } else {
        anyhow::bail!("dataset '{}' not found", id);
    }
    Ok(())
}

async fn dataset_show(id: &str, hash: &str) -> Result<()> {
    let (store, project) = get_store_and_project().await?;
    if let Some(ds) = dust::dataset::Dataset::from_hash(&store, &project, id, hash).await? {
        println!("dataset: {}", ds.dataset_id());
        println!("hash: {}", ds.hash());
        println!("created: {}", ds.created());
        println!("len: {}", ds.len());
        println!("keys: {:?}", ds.keys());
    } else {
        anyhow::bail!("dataset '{}' with hash '{}' not found", id, hash);
    }
    Ok(())
}

async fn probe_openai(key: &str) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()?;
    let r = client
        .get("https://api.openai.com/v1/models")
        .bearer_auth(key)
        .send()
        .await?;
    if r.status().is_success() {
        Ok(())
    } else {
        anyhow::bail!("status {}", r.status())
    }
}

async fn list_models() -> Result<()> {
    use std::env;

    println!("Available models for configured providers:\n");

    // Check OpenAI
    if let Ok(key) = env::var("OPENAI_API_KEY") {
        if !key.is_empty() {
            println!("OpenAI:");
            match fetch_openai_models(&key).await {
                Ok(models) => {
                    for model in models {
                        println!("  - {}", model);
                    }
                }
                Err(e) => {
                    eprintln!("  Error fetching models: {}", e);
                }
            }
            println!();
        }
    }

    // Check Anthropic
    if let Ok(key) = env::var("ANTHROPIC_API_KEY") {
        if !key.is_empty() {
            println!("Anthropic:");
            match fetch_anthropic_models(&key).await {
                Ok(models) => {
                    for model in models {
                        println!("  - {}", model);
                    }
                }
                Err(e) => {
                    eprintln!("  Error fetching models: {}", e);
                }
            }
            println!();
        }
    }

    Ok(())
}

async fn fetch_openai_models(key: &str) -> Result<Vec<String>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;
    let r = client
        .get("https://api.openai.com/v1/models")
        .bearer_auth(key)
        .send()
        .await?;

    if !r.status().is_success() {
        return Err(anyhow!("API returned status {}", r.status()));
    }

    let body: serde_json::Value = r.json().await?;
    let mut models = Vec::new();

    if let Some(data) = body.get("data").and_then(|d| d.as_array()) {
        for item in data {
            if let Some(id) = item.get("id").and_then(|i| i.as_str()) {
                // Filter to chat/completion models
                if id.starts_with("gpt-") || id.starts_with("o1-") || id.starts_with("o3-") {
                    models.push(id.to_string());
                }
            }
        }
    }

    models.sort();
    models.dedup();
    Ok(models)
}

async fn fetch_anthropic_models(key: &str) -> Result<Vec<String>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;
    let r = client
        .get("https://api.anthropic.com/v1/models")
        .header("x-api-key", key)
        .header("anthropic-version", "2023-06-01")
        .send()
        .await?;

    if !r.status().is_success() {
        return Err(anyhow!("API returned status {}", r.status()));
    }

    let body: serde_json::Value = r.json().await?;
    let mut models = Vec::new();

    // Check both "data" array format and "models" array format
    let items = body
        .get("data")
        .or_else(|| body.get("models"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| anyhow!("Unexpected API response format"))?;

    for item in items {
        if let Some(id) = item.get("id").and_then(|i| i.as_str()) {
            models.push(id.to_string());
        } else if let Some(name) = item.get("name").and_then(|n| n.as_str()) {
            models.push(name.to_string());
        } else if let Some(model) = item.as_str() {
            models.push(model.to_string());
        }
    }

    models.sort();
    Ok(models)
}

async fn probe_anthropic(key: &str) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()?;
    let r = client
        .get("https://api.anthropic.com/v1/models")
        .header("x-api-key", key)
        .header("anthropic-version", "2023-06-01")
        .send()
        .await?;
    if r.status().is_success() {
        Ok(())
    } else {
        anyhow::bail!("status {}", r.status())
    }
}

async fn spec_fmt(path: &str) -> Result<()> {
    let mut spec =
        fs::read_to_string(path).with_context(|| format!("Failed to read '{}'", path))?;
    // Normalize line endings and collapse multiple blank lines; ensure trailing newline.
    spec = spec.replace("\r\n", "\n");
    let collapsed = regex::Regex::new("\n{3,}")
        .unwrap()
        .replace_all(&spec, "\n\n")
        .to_string();
    let out = if collapsed.ends_with('\n') {
        collapsed
    } else {
        format!("{}\n", collapsed)
    };
    // Conservative formatting only (line endings, blank lines). Key reordering disabled for now.
    // Validate parse
    let _ = dust::app::App::new(&out)
        .await
        .context("spec parse failed after fmt")?;
    fs::write(path, out).with_context(|| format!("Failed to write '{}'", path))?;
    println!("Formatted: {}", path);
    Ok(())
}

async fn spec_hash(path: &str) -> Result<()> {
    let spec = fs::read_to_string(path).with_context(|| format!("Failed to read '{}'", path))?;
    let app = dust::app::App::new(&spec)
        .await
        .with_context(|| "parse failed")?;
    println!("{}", app.hash());
    Ok(())
}

async fn spec_diff(path: &str, against: Option<&str>) -> Result<()> {
    use similar::{Algorithm, TextDiff};
    let a = fs::read_to_string(path).with_context(|| format!("Failed to read '{}'", path))?;
    let b = if let Some(p) = against {
        fs::read_to_string(p).with_context(|| format!("Failed to read '{}'", p))?
    } else {
        // Compare current vs formatted output
        let mut tmp = a.replace("\r\n", "\n");
        tmp = regex::Regex::new("\n{3,}")
            .unwrap()
            .replace_all(&tmp, "\n\n")
            .to_string();
        if !tmp.ends_with('\n') {
            tmp.push('\n');
        }
        tmp
    };
    if a == b {
        println!("(no changes)");
        return Ok(());
    }
    let diff = TextDiff::configure()
        .algorithm(Algorithm::Myers)
        .diff_lines(&a, &b);
    for op in diff.ops() {
        for change in diff.iter_changes(op) {
            let sign = match change.tag() {
                similar::ChangeTag::Delete => '-',
                similar::ChangeTag::Insert => '+',
                similar::ChangeTag::Equal => ' ',
            };
            print!("{}{}", sign, change);
        }
    }
    Ok(())
}

async fn spec_tokens(path: &str, model: &str) -> Result<()> {
    let spec = fs::read_to_string(path).with_context(|| format!("Failed to read '{}'", path))?;
    // Pick tokenizer based on model
    use dust::providers::tiktoken::tiktoken as tk;
    let bpe = match model {
        m if m.contains("gpt-4o") || m.contains("gpt-4o-mini") || m.contains("gpt-4.1") => {
            tk::o200k_base_singleton()
        }
        m if m.contains("gpt-4") || m.contains("gpt-3.5") => tk::cl100k_base_singleton(),
        _ => tk::cl100k_base_singleton(),
    };
    // Collect prompts/instructions strings heuristically from the spec text
    let mut texts: Vec<(String, String)> = Vec::new();
    let spec_nc = util::strip_code_fences(&spec);
    for line in spec_nc.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("prompt:") {
            let v = trimmed
                .trim_start_matches("prompt:")
                .trim()
                .trim_matches('"')
                .to_string();
            if !v.is_empty() {
                texts.push(("prompt".to_string(), v));
            }
        }
        if trimmed.starts_with("instructions:") {
            let v = trimmed
                .trim_start_matches("instructions:")
                .trim()
                .trim_matches('"')
                .to_string();
            if !v.is_empty() {
                texts.push(("instructions".to_string(), v));
            }
        }
    }
    let mut total = 0usize;
    for (label, text) in texts {
        let tokens = tk::encode_async(bpe.clone(), &text)
            .await
            .unwrap_or_default();
        println!("{} {}", label, tokens.len());
        total += tokens.len();
    }
    println!("total {}", total);
    Ok(())
}

async fn auth_status() -> Result<()> {
    let mut any = false;
    for k in [
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "SERP_API_KEY",
        "BROWSERLESS_API_KEY",
    ] {
        if std::env::var(k).is_ok() {
            println!("[ok] {} configured", k);
            any = true;
        } else {
            println!("[ ] {} not set", k);
        }
    }
    if !any {
        eprintln!("No providers configured. Use 'dust auth set' or export env vars.");
    }
    Ok(())
}

async fn auth_list() -> Result<()> {
    let path = pg::cli_data_dir()?.join("credentials.json");
    if let Ok(bytes) = std::fs::read(&path) {
        if let Ok(map) = serde_json::from_slice::<std::collections::HashMap<String, String>>(&bytes)
        {
            for k in map.keys() {
                println!("{}", k);
            }
            return Ok(());
        }
    }
    println!("(no local credentials at {})", path.display());
    Ok(())
}

async fn env_init() -> Result<()> {
    let content = "# Example environment (do not commit secrets)\n\
CORE_DATABASE_URI=postgres://dev:dev@127.0.0.1:54330/dust\n\
OPENAI_API_KEY=\n\
ANTHROPIC_API_KEY=\n\
SERP_API_KEY=\n\
BROWSERLESS_API_KEY=\n";
    fs::write(".env.example", content).context("failed to write .env.example")?;
    println!("Wrote .env.example");
    Ok(())
}

async fn auth_set(pairs: &[String]) -> Result<()> {
    let mut map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let path = pg::cli_data_dir()?.join("credentials.json");
    if let Ok(bytes) = std::fs::read(&path) {
        if let Ok(existing) =
            serde_json::from_slice::<std::collections::HashMap<String, String>>(&bytes)
        {
            map = existing;
        }
    }
    for p in pairs {
        let mut sp = p.splitn(2, '=');
        let k = sp.next().unwrap_or("").trim().to_string();
        let v = sp.next().unwrap_or("").to_string();
        if !k.is_empty() {
            map.insert(k, v);
        }
    }
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&path, serde_json::to_vec_pretty(&map)?).context("write credentials")?;
    println!("Updated {} keys", map.len());
    Ok(())
}

use std::time::{Duration, SystemTime};

async fn dev_watch(args: RunArgs) -> Result<()> {
    // Determine primary file to watch
    let watch_label = if let Some(sp) = &args.spec_path {
        sp.clone()
    } else {
        args.assistant_file
            .clone()
            .unwrap_or_else(|| "(stdin)".to_string())
    };
    println!(
        "Watching {}{}",
        watch_label,
        args.messages_file
            .as_deref()
            .map(|m| format!(" and {}", m))
            .unwrap_or_default()
    );
    let mut last_primary = if let Some(sp) = &args.spec_path {
        file_mtime(sp)?
    } else if let Some(af) = &args.assistant_file {
        file_mtime(af)?
    } else {
        None
    };
    let mut last_msg = if let Some(m) = args.messages_file.as_deref() {
        file_mtime(m).ok()
    } else {
        None
    };
    // Initial run
    let _ = run_once(args.clone()).await;
    loop {
        tokio::time::sleep(Duration::from_millis(300)).await;
        let primary_now = if let Some(sp) = &args.spec_path {
            file_mtime(sp)?
        } else if let Some(af) = &args.assistant_file {
            file_mtime(af)?
        } else {
            None
        };
        let msg_now = if let Some(m) = args.messages_file.as_deref() {
            file_mtime(m).ok()
        } else {
            None
        };
        let mut changed = false;
        if primary_now > last_primary {
            last_primary = primary_now;
            changed = true;
        }
        if msg_now.is_some() && msg_now > last_msg {
            last_msg = msg_now;
            changed = true;
        }
        if changed {
            println!("[dev] change detected, re-running...");
            let _ = run_once(args.clone()).await;
        }
    }
}

fn file_mtime(p: &str) -> Result<Option<SystemTime>> {
    Ok(Some(
        std::fs::metadata(p)
            .with_context(|| format!("stat '{}': not found", p))?
            .modified()?,
    ))
}

async fn cache_cmd(cmd: CacheCmd) -> Result<()> {
    match cmd {
        CacheCmd::Stats => {
            cache_stats().await?;
        }
        CacheCmd::Clear {
            provider: _,
            before,
        } => {
            cache_clear(before.as_deref()).await?;
        }
        CacheCmd::Export {
            file,
            before,
            r#type,
        } => {
            cache_export(&file, before.as_deref(), r#type.as_deref()).await?;
        }
        CacheCmd::Import { file } => {
            cache_import(&file).await?;
        }
    }
    Ok(())
}

async fn cache_stats() -> Result<()> {
    let (store, project) = get_store_and_project().await?;
    let pool = store.raw_pool().clone();
    let c = pool.get().await?;
    let stmt = c
        .prepare("SELECT type, COUNT(*) FROM cache WHERE project = $1 GROUP BY type ORDER BY type")
        .await?;
    let rows = c.query(&stmt, &[&project.project_id()]).await?;
    if rows.is_empty() {
        println!("(empty)");
        return Ok(());
    }
    for row in rows {
        let t: String = row.get(0);
        let n: i64 = row.get(1);
        println!("{} {}", t, n);
    }
    Ok(())
}

async fn cache_clear(before: Option<&str>) -> Result<()> {
    let (store, project) = get_store_and_project().await?;
    let pool = store.raw_pool().clone();
    let c = pool.get().await?;
    if let Some(date) = before {
        let ts = parse_date_to_epoch(date)? as i64;
        let stmt = c
            .prepare("DELETE FROM cache WHERE project = $1 AND created < $2")
            .await?;
        let r = c.execute(&stmt, &[&project.project_id(), &ts]).await?;
        println!("deleted {} rows (before {})", r, date);
    } else {
        let stmt = c.prepare("DELETE FROM cache WHERE project = $1").await?;
        let r = c.execute(&stmt, &[&project.project_id()]).await?;
        println!("deleted {} rows", r);
    }
    Ok(())
}

fn parse_date_to_epoch(s: &str) -> Result<u64> {
    // Accept YYYY-MM-DD; interpret as UTC midnight
    let d = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")?;
    let dt = d
        .and_hms_opt(0, 0, 0)
        .expect("midnight should always be valid");
    Ok(dt.and_utc().timestamp() as u64)
}

async fn cache_export(path: &str, before: Option<&str>, typ: Option<&str>) -> Result<()> {
    let (store, project) = get_store_and_project().await?;
    let pool = store.raw_pool().clone();
    let c = pool.get().await?;
    let mut q = String::from(
        "SELECT created, hash, request, response, type, version FROM cache WHERE project = $1",
    );
    let pid = project.project_id();
    let rows = match (before, typ) {
        (None, None) => {
            let stmt = c.prepare(&(q.clone() + " ORDER BY created")).await?;
            c.query(&stmt, &[&pid]).await?
        }
        (Some(b), None) => {
            q.push_str(" AND created < $2 ORDER BY created");
            let ts: i64 = parse_date_to_epoch(b)? as i64;
            let stmt = c.prepare(&q).await?;
            c.query(&stmt, &[&pid, &ts]).await?
        }
        (None, Some(t)) => {
            q.push_str(" AND type = $2 ORDER BY created");
            let stmt = c.prepare(&q).await?;
            c.query(&stmt, &[&pid, &t]).await?
        }
        (Some(b), Some(t)) => {
            q.push_str(" AND created < $2 AND type = $3 ORDER BY created");
            let ts: i64 = parse_date_to_epoch(b)? as i64;
            let stmt = c.prepare(&q).await?;
            c.query(&stmt, &[&pid, &ts, &t]).await?
        }
    };
    let mut f = std::fs::File::create(path).with_context(|| format!("create {}", path))?;
    for row in rows {
        let created: i64 = row.get(0);
        let hash: String = row.get(1);
        let request: String = row.get(2);
        let response: String = row.get(3);
        let typ: String = row.get(4);
        let ver: i32 = row.get(5);
        let v = serde_json::json!({
            "export_schema": "cache_export_v1",
            "created": created,
            "hash": hash,
            "request": request,
            "response": response,
            "type": typ,
            "version": ver
        });
        use std::io::Write;
        writeln!(f, "{}", serde_json::to_string(&v)?).ok();
    }
    println!("exported cache entries to {}", path);
    Ok(())
}

async fn cache_import(path: &str) -> Result<()> {
    let (store, project) = get_store_and_project().await?;
    let pool = store.raw_pool().clone();
    let c = pool.get().await?;
    let stmt = c.prepare("INSERT INTO cache (id, project, created, hash, request, response, type, version) VALUES (DEFAULT, $1, $2, $3, $4, $5, $6, $7)").await?;
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path))?;
    let mut n = 0u64;
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let v: serde_json::Value =
            serde_json::from_str(line).with_context(|| "invalid NDJSON line")?;
        let created: i64 = v.get("created").and_then(|x| x.as_i64()).unwrap_or(0);
        let hash = v.get("hash").and_then(|x| x.as_str()).unwrap_or("");
        let request = v.get("request").and_then(|x| x.as_str()).unwrap_or("");
        let response = v.get("response").and_then(|x| x.as_str()).unwrap_or("");
        let typ = v.get("type").and_then(|x| x.as_str()).unwrap_or("");
        let ver: i32 = v.get("version").and_then(|x| x.as_i64()).unwrap_or(0) as i32;
        let _ = c
            .query_one(
                &stmt,
                &[
                    &project.project_id(),
                    &created,
                    &hash,
                    &request,
                    &response,
                    &typ,
                    &ver,
                ],
            )
            .await;
        n += 1;
    }
    println!("imported {} cache entries from {}", n, path);
    Ok(())
}

async fn db_cmd(cmd: DbCmd) -> Result<()> {
    match cmd {
        // Start/Stop/Status don't make sense when DB lifecycle matches CLI lifecycle
        // DbCmd::Start { .. } => {
        //     // Database starts automatically when needed
        // }
        // DbCmd::Stop => {
        //     // Database stops when the CLI exits
        // }
        // DbCmd::Status => {
        //     println!("{}", pg::db_status());
        // }
        // Backup and Reset still make sense for data management
        DbCmd::Backup { out } => {
            pg::backup(&out)?;
            println!("[ok] backup written: {}", out);
        }
        DbCmd::Reset { keep_backup } => {
            pg::reset(keep_backup)?;
            println!(
                "[ok] reset data dir{}",
                if keep_backup { " (kept backup)" } else { "" }
            );
        }
        _ => {
            anyhow::bail!(
                "This db command is not supported - database lifecycle matches CLI lifecycle"
            );
        }
    }
    Ok(())
}

async fn docs_cmd(cmd: DocsCmd) -> Result<()> {
    match cmd {
        DocsCmd::Index { path, incremental } => docs::index(&path, incremental),
        DocsCmd::Search { query, k } => docs::search(&query, k),
        DocsCmd::Clear => docs::clear(),
        DocsCmd::Stats => docs::stats(),
    }?;
    Ok(())
}

async fn dataset_cmd(cmd: DatasetCmd) -> Result<()> {
    match cmd {
        DatasetCmd::Create { id, file } => dataset_create(&id, &file).await,
        DatasetCmd::List => dataset_list().await,
        DatasetCmd::Hashes { id } => dataset_hashes(&id).await,
        DatasetCmd::Show { id, hash } => dataset_show(&id, &hash).await,
    }
}

async fn auth_cmd(cmd: AuthCmd) -> Result<()> {
    match cmd {
        AuthCmd::Status => auth_status().await,
        AuthCmd::List => auth_list().await,
        AuthCmd::EnvInit => env_init().await,
        AuthCmd::Set { pairs } => auth_set(&pairs).await,
    }
}

async fn block_cmd(cmd: BlockCmd) -> Result<()> {
    match cmd {
        BlockCmd::Add {
            spec_path,
            block_type,
            name,
        } => block_add(&spec_path, &block_type, &name).await,
        BlockCmd::Rm { spec_path, name } => block_rm(&spec_path, &name).await,
        BlockCmd::Rename {
            spec_path,
            old,
            new,
        } => block_rename(&spec_path, &old, &new).await,
        BlockCmd::Move {
            spec_path,
            name,
            before,
            after,
        } => block_move(&spec_path, &name, before.as_deref(), after.as_deref()).await,
        BlockCmd::SetConfig {
            spec_path,
            name,
            kv,
        } => block_set_config(&spec_path, &name, &kv).await,
    }
}

async fn block_add(spec_path: &str, block_type: &str, name: &str) -> Result<()> {
    let mut spec =
        fs::read_to_string(spec_path).with_context(|| format!("Failed to read '{}'", spec_path))?;
    let snippet = match block_type.to_lowercase().as_str() {
        "chat" => format!("\n\nchat {} {{\n  temperature: 0.7\n  messages_code:\n```\n_fun = (env) => {{\n  // return [{{ role: \"user\", content: \"hi!\"}}];\n}}\n```\n}}\n", name),
        "llm" => format!("\n\nllm {} {{\n  temperature: 0.7\n  max_tokens: 64\n  prompt: Hello world\n}}\n", name),
        "code" => format!("\n\ncode {} {{\n  code:\n```\n_fun = (env) => ({{ ok: true }})\n```\n}}\n", name),
        "curl" => format!("\n\ncurl {} {{\n  scheme: \"https\",\n  method: \"GET\",\n  url: \"https://example.com\",\n  headers_code:\n```\n_fun = (env) => ({{}})\n```\n  body_code:\n```\n_fun = (env) => \"\"\n```\n}}\n", name),
        _ => bail!("unsupported block type: {}", block_type),
    };
    spec.push_str(&snippet);

    // Validate with Core before writing
    block_validation::validate_and_write(spec_path, &spec).await?;
    println!("Appended {} block '{}' to {}", block_type, name, spec_path);
    Ok(())
}

fn build_chat_spec_from_assistant(cfg: &Value, context: Option<String>) -> String {
    let model = cfg
        .get("model")
        .and_then(|m| m.get("model"))
        .and_then(|s| s.as_str())
        .unwrap_or("gpt-4o");
    let instructions = cfg
        .get("instructions")
        .and_then(|s| s.as_str())
        .unwrap_or("You are a helpful assistant.");
    let instructions = if let Some(ctx) = context {
        format!(
            "{}\n\nContext (may be partial, cite when used):\n{}",
            instructions, ctx
        )
    } else {
        instructions.to_string()
    };
    // A minimal chat spec that reads messages from INPUT dataset
    format!(
        "\ninput INPUT {{}}\n\nchat ASSIST {{\n  model: \"{}\",\n  instructions: \"{}\",\n  messages_code:\n```\n(env) => env[\"state\"][\"INPUT\"][0][\"messages\"]\n```\n}}\n",
        model, instructions.replace('"', "\\\"")
    )
}

fn maybe_local_docs_context(cfg: &Value, messages: &Value) -> Result<Option<String>> {
    let enabled = cfg
        .get("tools")
        .and_then(|t| t.get("local_docs"))
        .and_then(|l| l.get("enable"))
        .and_then(|b| b.as_bool())
        .unwrap_or(false);
    if !enabled {
        return Ok(None);
    }
    let k = cfg
        .get("tools")
        .and_then(|t| t.get("local_docs"))
        .and_then(|l| l.get("k"))
        .and_then(|n| n.as_u64())
        .unwrap_or(3) as usize;
    let query = last_user_text(messages).unwrap_or_default();
    if query.trim().is_empty() {
        return Ok(None);
    }
    // Emit a simple orchestrator_plan event
    util::emit_event(
        "orchestrator_plan",
        serde_json::json!({"tools":["local_docs"],"reason":"query contains text; searching local docs","fallback":"direct_answer"}),
    );
    // Collect hits
    let hits = crate::docs::search_hits(&query, k).unwrap_or_default();
    let mut parts = Vec::new();
    for h in &hits {
        util::emit_event(
            "orchestrator_tool_result",
            serde_json::json!({"tool":"local_docs","ok":true,"path":h.path}),
        );
        parts.push(format!("- {}: {}", h.path, h.snippet));
    }
    if parts.is_empty() {
        return Ok(None);
    }
    Ok(Some(parts.join("\n")))
}

fn last_user_text(messages: &Value) -> Option<String> {
    match messages {
        Value::Array(arr) => {
            for item in arr.iter().rev() {
                if let Some(role) = item.get("role").and_then(|r| r.as_str()) {
                    if role == "user" {
                        if let Some(content) = item.get("content").and_then(|c| c.as_array()) {
                            for c in content {
                                if c.get("type").and_then(|t| t.as_str()) == Some("text") {
                                    if let Some(t) = c.get("text").and_then(|t| t.as_str()) {
                                        return Some(t.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
            None
        }
        Value::Object(o) => o.get("messages").and_then(last_user_text),
        _ => None,
    }
}

async fn estimate_total_tokens(spec: &str, messages: Option<&Value>, model: &str) -> Result<usize> {
    use dust::providers::tiktoken::tiktoken as tk;
    let bpe = match model {
        m if m.contains("gpt-4o") || m.contains("gpt-4.1") => tk::o200k_base_singleton(),
        m if m.contains("gpt-4") || m.contains("gpt-3.5") => tk::cl100k_base_singleton(),
        _ => tk::cl100k_base_singleton(),
    };
    let mut total = 0usize;
    // Count prompt/instructions lines outside fences
    let mut in_code = false;
    for line in spec.lines() {
        if line.trim_start().starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }
        let trimmed = line.trim();
        if trimmed.starts_with("prompt:") || trimmed.starts_with("instructions:") {
            let v = trimmed
                .split_once(':')
                .map(|x| x.1)
                .unwrap_or("")
                .trim()
                .trim_matches('"')
                .to_string();
            if !v.is_empty() {
                let t = tk::encode_async(bpe.clone(), &v).await.unwrap_or_default();
                total += t.len();
            }
        }
    }
    // Count messages plain text parts if provided
    if let Some(mv) = messages {
        total += estimate_message_tokens_inner(bpe.clone(), mv).await;
    }
    Ok(total)
}

async fn estimate_message_tokens_inner(
    bpe: std::sync::Arc<parking_lot::RwLock<dust::providers::tiktoken::tiktoken::CoreBPE>>,
    mv: &Value,
) -> usize {
    use dust::providers::tiktoken::tiktoken as tk;
    // Implement without recursion by iteratively descending into objects
    let mut stack: Vec<Value> = vec![mv.clone()];
    let mut tot = 0usize;
    while let Some(cur) = stack.pop() {
        match cur {
            Value::Array(arr) => {
                for item in arr {
                    if let Some(content) = item.get("content").and_then(|c| c.as_array()) {
                        for c in content {
                            if c.get("type").and_then(|t| t.as_str()) == Some("text") {
                                if let Some(tstr) = c.get("text").and_then(|t| t.as_str()) {
                                    let tks = tk::encode_async(bpe.clone(), tstr)
                                        .await
                                        .unwrap_or_default();
                                    tot += tks.len();
                                }
                            }
                        }
                    }
                }
            }
            Value::Object(o) => {
                if let Some(v) = o.get("messages") {
                    stack.push(v.clone());
                }
            }
            _ => {}
        }
    }
    tot
}

async fn block_rm(spec_path: &str, name: &str) -> Result<()> {
    let spec =
        fs::read_to_string(spec_path).with_context(|| format!("Failed to read '{}'", spec_path))?;
    let (new_spec, removed) = crate::block::remove_block_by_name(&spec, name)
        .ok_or_else(|| anyhow::anyhow!("block '{}' not found", name))?;

    // Validate with Core before writing
    block_validation::validate_and_write(spec_path, &new_spec).await?;
    println!("Removed block '{}' ({} bytes)", name, removed.len());
    Ok(())
}

async fn block_rename(spec_path: &str, old: &str, new: &str) -> Result<()> {
    let spec =
        fs::read_to_string(spec_path).with_context(|| format!("Failed to read '{}'", spec_path))?;
    let (idx, _) = crate::block::find_block_bounds(&spec, old)
        .ok_or_else(|| anyhow::anyhow!("block '{}' not found", old))?;

    // Check if new name already exists
    if crate::block::find_block_bounds(&spec, new).is_some() {
        anyhow::bail!("block '{}' already exists", new);
    }
    let mut out = String::with_capacity(spec.len());
    out.push_str(&spec[..idx.start]);
    // Replace the name token on the header line
    let header = &spec[idx.start..idx.header_end];
    let replaced = header.replacen(&format!(" {} ", old), &format!(" {} ", new), 1);
    out.push_str(&replaced);
    out.push_str(&spec[idx.header_end..]);

    // Validate with Core before writing
    block_validation::validate_and_write(spec_path, &out).await?;
    println!("Renamed block '{}' -> '{}'", old, new);
    Ok(())
}

async fn block_move(
    spec_path: &str,
    name: &str,
    before: Option<&str>,
    after: Option<&str>,
) -> Result<()> {
    if before.is_some() && after.is_some() {
        anyhow::bail!("specify only one of --before or --after");
    }
    let spec =
        fs::read_to_string(spec_path).with_context(|| format!("Failed to read '{}'", spec_path))?;
    let (src_bounds, snippet) = crate::block::find_and_extract(&spec, name)
        .ok_or_else(|| anyhow::anyhow!("block '{}' not found", name))?;
    let mut base = String::new();
    base.push_str(&spec[..src_bounds.start]);
    base.push_str(&spec[src_bounds.end..]);
    let insert_at = if let Some(bn) = before {
        if let Some((dst_bounds, _)) = crate::block::find_and_extract(&base, bn) {
            dst_bounds.start
        } else {
            anyhow::bail!("target '{}' not found", bn)
        }
    } else if let Some(an) = after {
        if let Some((dst_bounds, _)) = crate::block::find_and_extract(&base, an) {
            dst_bounds.end
        } else {
            anyhow::bail!("target '{}' not found", an)
        }
    } else {
        anyhow::bail!("either --before or --after required")
    };
    let mut out = String::new();
    out.push_str(&base[..insert_at]);
    out.push_str(&snippet);
    out.push_str(&base[insert_at..]);

    // Validate with Core before writing
    block_validation::validate_and_write(spec_path, &out).await?;
    println!("Moved block '{}'", name);
    Ok(())
}

async fn block_set_config(spec_path: &str, name: &str, kv: &[String]) -> Result<()> {
    let spec =
        fs::read_to_string(spec_path).with_context(|| format!("Failed to read '{}'", spec_path))?;
    let (bounds, _) = crate::block::find_block_bounds(&spec, name)
        .ok_or_else(|| anyhow::anyhow!("block '{}' not found", name))?;
    let mut body = spec[bounds.header_end..bounds.end].to_string();
    for pair in kv {
        let mut sp = pair.splitn(2, '=');
        let k = sp.next().unwrap_or("").trim();
        let v = sp.next().unwrap_or("").trim();
        if k.is_empty() {
            continue;
        }
        body = crate::block::set_config_line(&body, k, v);
    }
    let mut out = String::new();
    out.push_str(&spec[..bounds.header_end]);
    out.push_str(&body);
    out.push_str(&spec[bounds.end..]);

    // Validate with Core before writing
    block_validation::validate_and_write(spec_path, &out).await?;
    println!("Updated config for '{}'", name);
    Ok(())
}

// Note: helper implementations for spec/block manipulation now live in
// crate::util and crate::block to avoid duplication and keep main.rs focused.
