# dustx — Local‑First Dust CLI

A fast, local‑first CLI to author, validate, and run Dust specs without the SaaS UI. It uses Dust Core as a library, manages an embedded Postgres locally, and favors file‑based workflows.

## Install / Build

- Requirements:
  - Rust 1.87+ (for embedded Postgres feature)
  - macOS 13+/arm64/x86_64 or Ubuntu 22.04+
- Build:
  - `cd cli-rs`
  - Optional embedded Postgres: `cargo build --features pg-embedded`
  - Without embedded PG: `cargo build`

Notes:
- If you don’t enable `pg-embedded`, set `CORE_DATABASE_URI` to a local Postgres.
- On some systems, you may need to select Rust 1.87 explicitly (e.g., `cargo +1.87.0 build`).

## Quickstart

- Check a spec:
  - `dustx spec check path/to/app.dust`
- Lint a spec (warnings + errors with line numbers):
  - `dustx spec lint path/to/app.dust`
- Format a spec (conservative canonicalization):
  - `dustx spec fmt path/to/app.dust`
- Create a dataset:
  - `dustx dataset create emails ./fixtures/messages.json`
- Run a spec with dataset:
  - `dustx run path/to/app.dust --dataset-id emails --dataset-hash <hash>`
- Assistant config (chat via messages file):
  - `dustx run --assistant-file assistant.json --messages-file messages.json`
- Watch mode:
  - `dustx dev --spec path/to/app.dust --messages-file messages.json`

## Credentials

- Local plaintext creds: `.dust/credentials.json`. Set via:
  - `dustx auth set --env OPENAI_API_KEY=... --env ANTHROPIC_API_KEY=...`
- Env vars take precedence. The CLI loads `.dust/credentials.json` on startup (opt‑in convenience).

## What’s Supported in This Release

- Blocks: Input, Data, LLM/Chat, Code (JS), Curl, Search (SerpAPI/Serper), Browser (Browserless), Map/Reduce/While/End.
- Block authoring helpers: `block add|rm|rename|move|set-config`.
- Datasets: `create|list|hashes|show`.
- Spec tools: `check|lint|fmt|diff|hash|tokens`.
- Cache: `stats|clear|export|import`.
- Doctor: environment + DB checks with helpful hints.

## What’s Not Supported in This Release

- DataSource / Database / DatabaseSchema blocks. The CLI will fail fast if they are present. Use `docs` commands (local_docs) for early RAG instead.

## Tips

- Token/cost preflight:
  - `run --max-total-tokens N` emits a `token_estimate` (and `cost_estimate` if `DUST_COST_PER_1K_TOKENS_USD` is set) and aborts if over cap.
- Events: All structured events are printed as NDJSON, one per line.
- Embedded PG: managed under `.dust/`; controls available via `dustx db start|status|stop|backup|reset`.

## Troubleshooting

- `doctor` exits nonzero for blocking issues (e.g., embedded DB failures or `.dust` not writable). It prints hints with suggested fixes.
- If provider probes warn, it does not block most flows (unless your spec uses those providers).

