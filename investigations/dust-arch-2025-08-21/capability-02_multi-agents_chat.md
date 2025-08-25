Title: Multi-Agents, Provider-Agnostic Chat (Assistants)

Scope
- Configure and orchestrate Assistants (agents) with tools and model selection, independent of provider.
- Support synchronous runs with streaming and seamless transition to durable async via Temporal.

Key Code
- Orchestration: `front/lib/api/assistant/*` (notably `agent.ts`, `conversation.ts`, `messages.ts`, `generation.ts`, streaming helpers)
- Temporal execution: `front/temporal/agent_loop/*` (workflows, activities for model/tool steps)
- Model selection: `front/pages/api/w/[wId]/models.ts`, `front/lib/assistant.ts`, `components/providers/types`
- Tools: MCP actions (`front/lib/actions/mcp*`), internal action servers, extension-side MCP (`extension/platforms/front/*`)

Flow
1) User sends a message (Front API routes under `/api/w/[wId]/assistant/...`).
2) Orchestration selects model (plan/flags/whitelist) and tool set; emits streaming events.
3) Sync path tries to complete within a threshold; if long-running tools/timeouts occur, it launches Temporal workflow with a saved state (`SyncTimeoutError` logic in `agent.ts`).
4) Temporal `agent_loop` executes steps reliably (LLM plan/action, tool run, result incorporation), heartbeating and resuming as needed.

Provider-Agnosticity
- Models exposed via `USED_MODEL_CONFIGS` and filtered by plan/flags (`canUseModel`).
- LLM invocation can be via Core (Dust Spec blocks) or via Front-managed tool flows.
- MCP tools abstract external capabilities; internal actions provide curated high-trust tools.

Extensibility
- Add tools: implement MCP/internal action modules and register with workflows.
- Add models/providers: extend provider configs and gate by feature flags; ensure token accounting aligns.
- Add streaming adapters: extend `front/lib/api/assistant/streaming` to integrate new event schemas.

Operational Notes
- Temporal worker names relevant: `agent_loop`, `agent_schedule`.
- Metrics emitted around steps and durations; wake locks used to guard synchronous runs.

