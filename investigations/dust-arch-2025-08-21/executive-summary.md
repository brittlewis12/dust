Title: Executive Summary — Dust Architecture (2025-08-21)

Overview
- Dust comprises three primary components: Core (Rust engine), Front (Next.js control plane), and Connectors (TS ingestion plane), backed by Temporal for durability and Qdrant/Elasticsearch for retrieval.

Core (Rust)
- Implements the Dust Spec DSL with deterministic versioning, and executes workflows with typed blocks (chat, search, database, browser, etc.).
- Integrates with LLM/embedding providers, Qdrant for vectors, Elasticsearch for search, and GCS for storage.
- Exposes an HTTP API used by Front and tooling; enforces tenant separation in vector/search paths.

Front (Next.js)
- Provides Assistants (multi-agent) orchestration, streaming UX, WorkOS-based auth, governance, and internal APIs for connectors.
- Runs many Temporal workers (agent loop, upsert queues, data retention, permissions, usage, relocation, production checks, WorkOS events).
- Manages model selection and tools (MCP/internal) with sync execution that can transition to durable async.

Connectors (TypeScript)
- Owns provider-specific sync pipelines using Temporal, persists state in Postgres, and upserts content via Front internal API.
- Offers schedules, monitoring, and graceful termination controls for workflows.

Extensibility & Modularity
- Core: add blocks, providers, or stores via traits and parser dispatch.
- Front: add models/tools/workers; integrate new streaming and governance.
- Connectors: add new providers with dedicated workflows and schedules.

Self-Hosting Requirements
- Infra: Postgres, Redis, Qdrant, Elasticsearch, GCS (or alternative object store adaptation), Temporal namespaces (with TLS in prod), WorkOS for auth.
- Provider keys as needed (OpenAI/Anthropic/Azure OpenAI/Mistral/etc.).

Fit to Desired Capabilities
- The codebase aligns well with a modular capability breakdown (DSL, agents, durable execution, embeddings, vector/search stores, connectors, WorkOS auth).
- An unbiased taxonomy also highlights cross-cutting planes (execution/data/control/ingestion/provider/storage) and observability.

