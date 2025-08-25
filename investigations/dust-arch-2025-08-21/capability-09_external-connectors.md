Title: External Data Connectors

Scope
- Durable ingestion from third-party platforms (Slack, Notion, GitHub, Google Drive, Intercom, Zendesk, Salesforce, Snowflake, etc.).
- Normalize to Dust’s data model and upsert into data sources (eventually vectorized and indexed by Core/ES/Qdrant).

Key Code
- Project: `connectors/*` (TypeScript).
- Temporal: `connectors/src/lib/temporal.ts`, `temporal_monitoring.ts`, `temporal_schedules.ts`.
- Storage: Sequelize models and wrappers under `connectors/src/resources/storage/*` (Postgres).
- Upsert pipeline: `connectors/src/lib/data_sources.ts` calls Front internal API with workspace API key; Front then calls Core.
- Provider-specific: under `connectors/src/connectors/*` with OAuth and sync logic.

Auth/OAuth
- Core provides OAuth providers (`core/src/oauth/*`), secrets encryption, token refreshes.
- Front exposes user-facing OAuth endpoints and stores relations to workspaces.

Queues & Schedules
- Temporal manages long-running syncs, retries, and periodic schedules; helpers to terminate extraneous workflows.
- Monitoring emits structured logs and stats; failure classes tagged.

Operational Notes
- For connectors that need proxies (e.g., Snowflake), use `PROXY_*` envs.
- Connectors require DB + Redis + Temporal connectivity, and access to Front internal API.

