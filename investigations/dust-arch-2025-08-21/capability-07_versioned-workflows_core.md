Title: Versioned Workflows (Core)

Scope
- Deterministic versioning of blocks and apps; strict structural validation; streaming run state.

Key Code
- `core/src/app.rs`: parse, validate, hash, execute.
- `core/src/run.rs`: `Run`, `RunConfig`, `RunType`, `RunStatus`, `BlockStatus`.

Versioning
- Each `Block` contributes a stable hash via its config and code.
- App hash computed from ordered block hashes; used as version identifier.

Execution
- `RunConfig` governs concurrency, caching, dataset usage.
- `Secrets` holds credentials injected per run.
- Streaming events per block for UI/telemetry.

Parallel Testing & Repro
- App hash enables comparing runs across versions.
- Cached requests (e.g., embedders) use request-specific hashes.

Extensibility
- Add new control flow or block: implement `Block` and integrate in parser dispatch.
- Add stores/providers: trait-based; ensure deterministic hashing includes relevant config.

