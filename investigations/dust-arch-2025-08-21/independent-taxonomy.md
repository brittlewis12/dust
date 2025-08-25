Title: Independent Taxonomy of Dust (Unbiased)

Note
- This taxonomy is derived from code structure and execution surfaces only, ignoring any prior capability framing.

Planes
- Execution Plane: Temporal workflows and workers that perform long-running, resumable tasks across the product (agents, ingestion, maintenance, events).
- Data Plane: Rust Core engine that parses/executes specs, interfaces with vector/search stores, and provides HTTP endpoints for runs and data operations.
- Control Plane: Front (Next.js) APIs and UI that manage assistants, users/orgs/auth, configuration, feature flags, quotas, and internal APIs used by connectors.
- Ingestion Plane: Connectors project (TS) implementing provider-specific sync pipelines and schedules, persisting state in Postgres, and upserting content through Front.
- Provider Plane: LLM and embedding providers (OpenAI, Anthropic, Azure OpenAI, Mistral, Fireworks, Together, xAI, Google AI Studio) and OAuth-based SaaS providers (Slack, Notion, GitHub, etc.).
- Storage Plane: PostgreSQL (front/connectors), Redis (cache/locks), Elasticsearch (search), Qdrant (vectors), GCS (documents/tables), optional proxies.
- Client Interfaces: Web app (assistants UI), CLI (`cli`) with WorkOS device flow/API-key mode, browser extension (`extension`) with MCP tooling, SDKs (`sdks`).
- Observability: OpenTelemetry tracing in Core, logging/metrics in Front/Connectors, status page integration.

Responsibilities (by component)
- Core: deterministic spec execution; providers integration; tenants isolation policies for vector/search; OAuth backend; search/vector/index tooling and migrations.
- Front: assistant orchestration; streaming; session/auth; Temporal workers orchestration for product features; internal API surface between connectors and Core; governance and plan/limits.
- Connectors: external sync durability; schedules; storage of sync state; uniform upsert interface to Front.

Extensibility Paths
- New Block/Provider/Store in Core; New model/tool/worker in Front; New Connector/provider module with schedules in Connectors.

