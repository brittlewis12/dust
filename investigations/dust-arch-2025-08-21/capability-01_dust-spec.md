Title: Dust Spec (DSL) — Parser, Blocks, Engine

Scope
- Define, validate, and execute versioned, data-aware workflows as sequences of typed Blocks.
- Provide deterministic hashing for block/app versions and structured run-time state and events.

Key Code
- Grammar: `core/src/dust.pest`
- Blocks: `core/src/blocks/*` (chat, data_source, database, database_schema, search, browser, curl, code, data, input, map, reduce, while, end)
- Engine: `core/src/app.rs` (parsing, validation, execution), `core/src/run.rs` (run state, statuses)
- HTTP API: `core/bin/core_api.rs` (run apps, data source ops, search, DB features)

Execution Flow
1) Parse Spec: `DustParser` parses `dust` -> blocks with types, names, config.
2) Validate Structure:
   - At most one `input`.
   - Proper map/reduce, while/end pairing; no invalid nesting.
   - Unique names except paired blocks.
3) Hashing & Versioning:
   - Each block implements a hasher over its config and code.
   - App hash derives from ordered block hashes.
4) Run:
   - `App::run` creates `Run` with `RunConfig` and `Secrets`.
   - Blocks execute with `Env` (project, stores, credentials, qdrant clients, event sender).
   - Map/Reduce and While control flow handled explicitly in `app.rs`.
   - Emits per-block `BlockStatus` and streamed events.

Major Blocks (selected)
- `chat`: LLM prompting with provider-agnostic messaging.
- `data_source`: read/search against Dust data sources.
- `database`: SQL-like query over registered tables (local/remote). See `core/src/databases/*`.
- `search`: semantic search via Elasticsearch store.
- `browser`: scrape via Browserless API.
- `input`, `data`, `map`, `reduce`, `while`, `end`: control/data plumbing.

Providers
- LLM/Embedding providers via `core/src/providers/*` and traits (`llm.rs`, `embedder.rs`).
- Tokenizers: `tiktoken`, `sentencepiece` for model-specific token accounting.

Stores & External Systems
- Vector store: Qdrant (`core/src/data_sources/qdrant.rs`).
- Search store: Elasticsearch (`core/src/search_stores/search_store.rs`).
- Databases store: GCS-backed table storage (`core/src/databases_store/*`).
- Redis-based caches in some subsystems (e.g., OAuth, table upsert workers).

Extensibility
- New Block: implement `Block` in `core/src/blocks/`, extend parser in that block file, make it part of `parse_block`.
- New Provider/Model: extend `providers/*` and `provider.rs` maps; ensure tokenization support.
- New Store: implement trait (`stores::store::Store` or specific store traits) and wire into API.

APIs & Usage
- Core API binary (`core/bin/core_api.rs`): run specs, manage data sources, query search/index, database tables.
- Examples: `core/examples/*` specs show DSL structure and execution usage.

Operational Notes
- Deterministic hashing makes version diffing straightforward; pairs well with parallel testing.
- Care is taken for tenant isolation in vector/search via injected filters (see Qdrant section).

