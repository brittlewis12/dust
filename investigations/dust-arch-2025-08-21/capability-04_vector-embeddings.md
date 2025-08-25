Title: Vector Embeddings (Providers + Tokenization)

Scope
- Turn text into numerical vectors using provider-specific embedders with unified trait and retry semantics.

Key Code
- Trait and request: `core/src/providers/embedder.rs` (`Embedder`, `EmbedderRequest`, `EmbedderVector`).
- Providers: `core/src/providers/openai.rs`, `core/src/providers/mistral.rs` (plus additional provider files for LLMs).
- Tokenization: `core/src/providers/tiktoken/*`, `core/src/providers/sentencepiece/*`.

Models
- Supported (hard-coded map): `text-embedding-3-large-1536` (OpenAI), `mistral-embed` (Mistral).
- Mapping helpers: see `EmbedderProvidersModelMap`.

Behavior
- `EmbedderRequest::execute` initializes provider with credentials, retries with backoff (`with_retryable_back_off`).
- Tokenization utilities exposed for context-size checks and token usage estimation.

Credentials
- OpenAI: `OPENAI_API_KEY` (or more specific env in DS context)
- Mistral: `MISTRAL_API_KEY`
- Optional: Azure OpenAI fallback for embeddings (`OPENAI_EMBEDDINGS_AZURE_FALLBACK` paths)

Extensibility
- Add provider: implement `Embedder` for a model; wire through `provider()` registry and supported models map.
- Add tokenizer: include model-specific BPE/sentencepiece and bind in provider implementation.

