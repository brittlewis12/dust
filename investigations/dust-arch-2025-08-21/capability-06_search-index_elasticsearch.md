Title: Search Index — Elasticsearch

Scope
- Full-text and faceted search over data source nodes, tags, and data sources; filtering, sorting, pagination.

Key Code
- Search store trait + ES implementation: `core/src/search_stores/search_store.rs`.
- Index/backfill tools: `core/bin/elasticsearch/*`, `core/bin/migrations/*`.

Features
- Query DSL: builds bool queries with field/prefix/exact, optional source URL matching.
- Sorting: supports multi-field sort; maps `title` -> `title.keyword`.
- Pagination: page size up to 1000; cursor support.
- Tag aggregations: returns popular tags with counts and per-data-source breakdowns.
- Warnings: emits `TruncatedQueryClauses` when ES clause limits approached.

Environment
- `ELASTICSEARCH_URL`, `ELASTICSEARCH_USERNAME`, `ELASTICSEARCH_PASSWORD`.

Extensibility
- New fields/sorts: add mapping in `map_sort_field` and result shaping.
- New indices: add bin tools to create/backfill; maintain parity across versions.

