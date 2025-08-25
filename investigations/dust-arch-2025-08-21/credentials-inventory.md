Title: Credentials and Environment Inventory (Code-Derived)

Scope
- Enumerate credentials and environment variables referenced in code, grouped by component. Intentionally ignores any local `.env*` and `LOCAL.md` files.

Core (Rust)
- Vector Store (Qdrant): `QDRANT_CLUSTER_0_URL`, `QDRANT_CLUSTER_0_API_KEY` (and variants per cluster).
- Search (Elasticsearch): `ELASTICSEARCH_URL`, `ELASTICSEARCH_USERNAME`, `ELASTICSEARCH_PASSWORD` (API/backfills).
- OAuth Core: `OAUTH_API`, `OAUTH_API_KEY` (optional), `OAUTH_ENCRYPTION_KEY`.
  - Provider IDs/Secrets: Google Drive, GitHub (+private key path), Microsoft (+tools), Slack (+bot/tools), Intercom, Notion (+platform actions), Confluence, Zendesk, Jira, HubSpot, Freshworks, Monday, Gong, Salesforce. See `core/src/oauth/providers/*` for exact names (e.g., `OAUTH_SLACK_CLIENT_ID`, `OAUTH_GITHUB_APP_PRIVATE_KEY_PATH`).
- LLM/Embeddings: `OPENAI_API_KEY`, `AZURE_OPENAI_API_KEY`, `AZURE_OPENAI_ENDPOINT`, `ANTHROPIC_API_KEY`, `MISTRAL_API_KEY`, `TOGETHERAI_API_KEY`, `FIREWORKS_API_KEY`, `XAI_API_KEY`, `GOOGLE_CLOUD_PROJECT_ID`, `GOOGLE_CLOUD_LOCATION`, DS-specific overrides like `CORE_DATA_SOURCES_OPENAI_API_KEY`.
- Buckets (GCS): `DUST_DATA_SOURCES_BUCKET`, `DUST_TABLES_BUCKET`, `DUST_TABLE_UPDATES_BUCKET`, optional `GOOGLE_CLOUD_ACCESS_TOKEN`.
- Redis/DB/Proxy: `REDIS_URI`, optional `CORE_DATABASE_URI`, proxies `UNTRUSTED_EGRESS_PROXY_HOST/PORT`, `PROXY_*` for Snowflake.
- Telemetry/Logging: `OTEL_EXPORTER_OTLP_ENDPOINT`, `OTEL_LOG_LEVEL`, `RUST_LOG`.
- Misc: `DUST_FRONT_API`, `DUST_REGISTRY_SECRET`, `DISABLE_API_KEY_CHECK`, `API_KEYS`.

Front (Next.js)
- Database/Cache: `DATABASE_URL` (via config layers), `REDIS_URI`, `REDIS_CACHE_URI`.
- WorkOS Auth: `WORKOS_API_KEY`, `WORKOS_CLIENT_ID`, `WORKOS_COOKIE_PASSWORD`, `WORKOS_ISSUER_URL`, `WORKOS_WEBHOOK_SECRET`, `WORKOS_WEBHOOK_SIGNING_SECRET`, `WORKOS_SESSION_COOKIE_DOMAIN`, `WORKOS_ENVIRONMENT_ID`.
- Temporal: `TEMPORAL_CERT_PATH`, `TEMPORAL_CERT_KEY_PATH`, `TEMPORAL_NAMESPACE`, `TEMPORAL_AGENT_NAMESPACE`, `TEMPORAL_CONNECTORS_NAMESPACE`, `TEMPORAL_RELOCATION_NAMESPACE`.
- External Services: `SENDGRID_API_KEY`, `SENDGRID_*_TEMPLATE_ID`, `STRIPE_*`, `IPINFO_API_TOKEN`, `CUSTOMERIO_*`, `TEXT_EXTRACTION_URL`, `CONVERTAPI_API_KEY`.
- APIs: `CORE_API` (+ optional `CORE_API_KEY`), `CONNECTORS_API`, `DUST_CONNECTORS_SECRET`, `OAUTH_API` (+ optional `OAUTH_API_KEY`).
- Public Config: `NEXT_PUBLIC_DUST_CLIENT_FACING_URL`, analytics `NEXT_PUBLIC_GTM_TRACKING_ID`, `NEXT_PUBLIC_DATADOG_*`, `NEXT_PUBLIC_COMMIT_HASH`.
- Dust Apps: `DUST_APPS_WORKSPACE_ID`, `DUST_APPS_SPACE_ID`, `DUST_APPS_HELPER_DATASOURCE_VIEW_ID`, `DUST_DEVELOPMENT_SYSTEM_API_KEY`, `DUST_DEVELOPMENT_WORKSPACE_ID`, `DUST_REGISTRY_SECRET`.
- Misc: `SERVICE_ACCOUNT`, `REGION_RESOLVER_SECRET`, `DEBUG_PROFILER_SECRET`, `UNTRUSTED_EGRESS_PROXY_*`.

Connectors (TypeScript)
- Database/Cache: connector DB URI via `dbConfig.getRequiredDatabaseURI()` (commonly `DATABASE_URL`), `REDIS_URI`, `REDIS_CACHE_URI`.
- Temporal: `TEMPORAL_NAMESPACE` (plus certs in deployed envs).
- Provider-specific: proxy `PROXY_*` for Snowflake; others per connector type.
- Front Internal API: uses a workspace-scoped API key at runtime (provided via configuration, not env var).

Shared Operational
- Provision: Postgres, Redis, Qdrant, Elasticsearch, GCS, Temporal namespaces with TLS, WorkOS.
- Separate namespaces: agent/front/connectors/relocation; align workers with namespaces.

Notes
- This list is code-derived; some variables are optional/only used in production paths.
- LLM provider keys can be centralized or overridden per data source in some paths.

