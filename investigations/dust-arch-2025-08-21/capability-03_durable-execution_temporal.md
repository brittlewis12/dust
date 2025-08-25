Title: Durable Execution with Temporal

Scope
- Durable, cancelable workflows for agents, ingestion queues, maintenance tasks, and org events.

Key Code
- Front workers: `front/start_worker.ts` lists all worker entry points.
- Namespaces & connection: `front/lib/temporal.ts` (agent/front/connectors/relocation namespaces; TLS when deployed).
- Workflows/activities: `front/temporal/*` (agent_loop, agent_schedule, permissions_queue, upsert_queue, upsert_tables, usage_queue, data_retention, scrub_workspace, production_checks, relocation, remote_tools_sync, workos_events_queue, tracker).
- Connectors: `connectors/src/lib/temporal.ts`, `connectors/src/lib/temporal_monitoring.ts`, `connectors/src/lib/temporal_schedules.ts`.

Patterns
- Singletons for clients; TLS certs used in `production/staging` only.
- Activity heartbeats and cancellation checks; errors mapped to known workflow outcomes.
- Schedules: creation, triggering, termination, deletion utilities (connectors side).
- Monitoring: tags on activity/workflow failures for observability.

Namespaces
- `TEMPORAL_AGENT_NAMESPACE`, `TEMPORAL_NAMESPACE` (front), `TEMPORAL_CONNECTORS_NAMESPACE`, `TEMPORAL_RELOCATION_NAMESPACE`.

Operational Notes
- Local dev: plain connections; in prod/staging: `TEMPORAL_CERT_PATH`, `TEMPORAL_CERT_KEY_PATH` and namespace env are required.
- Workers run multiple queues concurrently; error isolation is done per worker.

