# Dust Apps Required for Self-Hosting

For a fully functional self-hosted Dust instance, you need the following Dust app specifications. These are hardcoded in `/lib/registry.ts` with specific app IDs and hashes.

## Critical Core Apps (Required for Basic Functionality)

### 1. **assistant-v2-multi-actions-agent**
- **Purpose**: Main orchestrator for multi-action assistants
- **App ID**: `0e9889c787`
- **Used by**: All assistant conversations with actions enabled
- **Critical**: YES - Without this, assistants cannot coordinate multiple tool calls

### 2. **assistant-v2-title-generator**
- **Purpose**: Generates conversation titles automatically
- **App ID**: `bPUvsPOQJb`
- **Used by**: Conversation title generation
- **Critical**: Medium - Conversations work without it but lack auto-titles

## MCP Server Dependencies (Required for Specific Tools)

### 3. **assistant-v2-reason**
- **Purpose**: Advanced reasoning for complex tasks
- **App ID**: `pDDBjLmlaD`
- **Used by**: `reasoning` MCP server when `advanced_reasoning` tool is called
- **Critical**: YES if using reasoning capabilities

### 4. **assistant-v2-process**
- **Purpose**: Structured data extraction from documents
- **App ID**: `b3vZ1HwBMO`
- **Used by**: `extract_data` MCP server for `extract_information_from_documents` tool
- **Critical**: YES if using data extraction features

### 5. **assistant-v2-query-tables**
- **Purpose**: SQL query generation for structured data
- **App ID**: `gMDXqLPfcE`
- **Used by**: `tables_query` MCP server for querying databases/spreadsheets
- **Critical**: YES if using table query features

### 6. **assistant-v2-visualization**
- **Purpose**: Data visualization and chart generation
- **App ID**: `tWcuYDj1OE`
- **Used by**: When visualization is enabled on an assistant
- **Critical**: YES if using data visualization features

## Legacy/Deprecated Apps (May Still Be Referenced)

### 7. **assistant-v2-retrieval**
- **Purpose**: Legacy retrieval implementation
- **App ID**: `qR8ugPw3gF`
- **Status**: Likely replaced by MCP search server
- **Critical**: NO - Modern retrieval uses MCP servers

### 8. **assistant-v2-websearch**
- **Purpose**: Legacy web search
- **App ID**: `3gv2P2HpKy`
- **Status**: Replaced by webtools MCP server
- **Critical**: NO - Modern web search is self-contained in MCP

### 9. **assistant-v2-browse**
- **Purpose**: Legacy web browsing
- **App ID**: `QuXLUot7iK`
- **Status**: Replaced by webtools MCP server
- **Critical**: NO - Modern browsing is self-contained in MCP

## Additional Support Apps

### 10. **conversation-file-summarizer**
- **Purpose**: Summarizes uploaded files in conversations
- **App ID**: `84dfc1d63c`
- **Used by**: File upload handling
- **Critical**: Medium - File uploads work without it but lack summaries

### 11. **suggest-agent-from-message**
- **Purpose**: Suggests which agent to use based on user message
- **App ID**: `GW0M6Aa3AP`
- **Used by**: Agent suggestion feature
- **Critical**: Low - Nice to have but not essential

## Document Tracker Apps (Enterprise Features)

### 12. **doc-tracker-retrieval**
- **Purpose**: Document tracking retrieval
- **App ID**: `jx0KRKXkWt`
- **Used by**: Document tracking hooks
- **Critical**: Only if using document tracking features

### 13. **doc-tracker-score-docs**
- **Purpose**: Score documents for tracking
- **App ID**: `DhKJLMbxEr`
- **Used by**: Document tracking hooks
- **Critical**: Only if using document tracking features

### 14. **doc-tracker-suggest-changes**
- **Purpose**: Suggest document changes
- **App ID**: `p3lQSJY96j`
- **Used by**: Document tracking hooks
- **Critical**: Only if using document tracking features

## Implementation Strategy for Self-Hosting

### Minimum Viable Setup
For basic assistant functionality, you absolutely need:
1. `assistant-v2-multi-actions-agent` - Core orchestrator
2. Any MCP-dependent apps for the tools you want to use:
   - `assistant-v2-reason` for reasoning
   - `assistant-v2-process` for data extraction
   - `assistant-v2-query-tables` for SQL queries
   - `assistant-v2-visualization` for charts

### Options for Implementation

1. **Recreate the Dust Apps**
   - Reverse engineer the prompts and logic
   - Create new app specs with the same functionality
   - Update the registry with your new app IDs and hashes

2. **Stub Out Non-Critical Apps**
   - Implement minimal versions that return empty/default responses
   - Focus effort on critical apps only

3. **Environment Variable Overrides**
   - Modify the code to allow environment variables to override app IDs
   - Point to your own implementations

4. **Direct MCP Implementation**
   - For some features, bypass the Dust app layer entirely
   - Implement the logic directly in the MCP servers
   - This is already done for search, web tools, file operations

## The Core Challenge

The main challenge is that these Dust app specifications contain:
- Carefully crafted prompts
- Specific input/output schemas
- Complex orchestration logic
- Model-specific optimizations

Without access to the original app specifications, you need to:
1. Understand what each app does (from code usage)
2. Recreate the functionality (prompts, logic, schemas)
3. Test thoroughly to ensure compatibility

## Registry Configuration

All apps are registered in `/lib/registry.ts` with:
```typescript
{
  app: {
    appId: string,        // Unique app identifier
    appHash: string,      // Content hash for caching
  },
  config: { ... }        // Runtime configuration
}
```

The system validates these IDs and hashes against the database, so you'll need to either:
- Seed your database with matching app records
- Modify the validation logic
- Use environment variables to override production values