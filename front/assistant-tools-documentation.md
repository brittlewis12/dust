# Assistant v2 Multi-Actions Agent - Available Tools

This document outlines the tools/actions that the `assistant-v2-multi-actions-agent` can call through the MCP (Model Context Protocol) system in Dust.

## Core Architecture

The assistant operates through MCP servers that expose tools as function calls. These servers are defined in `/lib/actions/mcp_internal_actions/servers/` and are instantiated based on the agent's configuration.

## Available MCP Servers and Their Tools

### 1. Search (`search`)
**Purpose**: Semantic search across configured data sources
- `semantic_search` - Search using semantic similarity with query, time frame, and optional tags
- `find_tags` - Discover available tags/labels (when dynamic tags enabled)

### 2. Web Tools (`web_search_&_browse`)
**Purpose**: Web interaction capabilities
- `websearch` - Google web search from query string
- `webbrowser` - Browse websites (supports multiple URLs, screenshots, markdown/HTML output)

### 3. Data Extraction (`extract_data`)
**Purpose**: Structured data extraction from documents
- `extract_information_from_documents` - Extract data matching a JSON schema from selected documents
- `find_tags` - Tag discovery (when dynamic tags enabled)

### 4. Include Data (`include_data`)
**Purpose**: Exhaustive document retrieval
- `retrieve_recent_documents` - Fetch most recent documents in reverse chronological order
- `find_tags` - Tag discovery (when dynamic tags enabled)

### 5. Tables Query (`query_tables` / `query_tables_v2`)
**Purpose**: Query structured data (spreadsheets, databases, Notion DBs)
- V1: `query_tables` - Single-step SQL query generation and execution
- V2:
  - `get_database_schema` - Retrieve database schema
  - `execute_database_query` - Execute SQL queries

### 6. Reasoning (`reasoning`)
**Purpose**: Advanced reasoning for complex tasks
- `advanced_reasoning` - Offload reasoning-heavy tasks to powerful models

### 7. Think (`think`)
**Purpose**: Planning and problem-solving
- `describe_plan` - Create detailed plans for complex problems

### 8. Run Dust App (`run_dust_app`)
**Purpose**: Execute configured Dust applications
- Dynamic tool name based on configured app
- Falls back to `run_dust_app` if no specific app configured

### 9. Run Agent (`run_agent`)
**Purpose**: Orchestrate child agents
- Dynamic tool names like `run_{agent_name}` based on configured child agent
- `run_agent_tool_not_available` when no child agent configured

### 10. File System Navigation (`data_sources_file_system`)
**Purpose**: Navigate and search file-like data sources
- `cat` - Read document contents by nodeId
- `find` - Find content by title pattern
- `list` - List directory contents (like Unix ls)
- `locate_in_tree` - Show path from node to root
- `semantic_search` - Search within specific nodes
- `find_tags` - Tag discovery (when dynamic tags enabled)

### 11. Conversation Files (`conversation_files`)
**Purpose**: Access files attached to conversations
- `conversation_include_file` - Include full content of attached files
- `conversation_list_files` - List all conversation attachments
- `cat` - Read large files with offset/limit and grep

### 12. File Generation (`file_generation`)
**Purpose**: Create and convert files
- `get_supported_source_formats_for_output_format` - Query format conversion support
- `convert_file_format` - Convert between file formats
- `generate_file` - Generate new files with specified content

### 13. Image Generation (`image_generation`)
**Purpose**: AI image creation
- `generate_image` - Generate images from text descriptions using DALL-E

### 14. Agent Memory (`agent_memory`)
**Purpose**: User-scoped persistent memory
- `retrieve` - Get all memories for current user
- `record_entries` - Add new memory entries
- `erase_entries` - Delete memory entries by index
- `edit_entries` - Modify existing memory entries

### 15. Agent Management (`agent_management`)
**Purpose**: Dynamic agent configuration
- `create_agent` - Create new agents with optional sub-agents

## Integration-Specific Servers

These require external service connections:
- **GitHub** (`github`) - Repository interaction
- **Notion** (`notion`) - Notion workspace operations
- **Slack** (`slack`) - Slack messaging
- **Gmail** (`gmail`) - Email operations
- **Google Calendar** (`google_calendar`) - Calendar management
- **Google Sheets** (`google_sheets`) - Spreadsheet operations
- **Salesforce** (`salesforce`) - CRM operations
- **HubSpot** (`hubspot`) - Marketing/CRM operations
- **Jira** (`jira`) - Issue tracking
- **Monday** (`monday`) - Project management
- **Outlook** (`outlook`) - Email operations
- **Outlook Calendar** (`outlook_calendar`) - Calendar operations
- **Freshservice** (`freshservice`) - IT service management

## Tool Availability Modes

Tools are configured with different availability patterns:

1. **Auto-available**: Core tools automatically available to all assistants
   - Search, web tools, include, reasoning, file/image generation, agent memory

2. **Manual configuration**: Require explicit setup in assistant builder
   - Integration-specific tools (GitHub, Notion, etc.)

3. **Hidden from builder**: Internal/system tools
   - Agent router, missing action catcher, primitive types debugger

4. **Feature-flagged**: Experimental or beta features
   - Think server, some integrations

## How Tools Are Selected

The assistant determines which tools to use based on:

1. **Agent Configuration**: Which MCP servers are configured for the agent
2. **User Request**: Natural language understanding of the task
3. **Context**: Conversation history and available data sources
4. **Tool Availability**: What tools are accessible given the current configuration

## Tool Invocation Flow

1. User sends message to assistant
2. Assistant analyzes request and available tools
3. Assistant generates tool calls through MCP protocol
4. MCP servers execute tools and return results
5. Assistant processes results and may chain additional tool calls
6. Final response generated for user

## Key Implementation Files

- Tool definitions: `/lib/actions/mcp_internal_actions/servers/[server_name].ts`
- Tool constants: `/lib/actions/mcp_internal_actions/constants.ts`
- MCP server registry: `/lib/actions/mcp_internal_actions/servers/index.ts`
- Tool input schemas: `/lib/actions/mcp_internal_actions/input_schemas.ts`
- Tool output schemas: `/lib/actions/mcp_internal_actions/output_schemas.ts`

## Notes for Local Development

When running locally, the assistant needs:
1. The MCP servers to be properly registered
2. API keys for external services (stored in environment or database)
3. Proper data source configurations
4. The core Dust app (`assistant-v2-multi-actions-agent`) to orchestrate tool calls

The actual tool selection and orchestration logic is embedded in the Dust app's prompt and behavior, which would need to be recreated or approximated for local development.
