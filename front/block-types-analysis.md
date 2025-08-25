# Dust App Block Types Analysis

## Block Components Review

Based on analysis of `/Users/britt/code/dust/front/components/app/blocks/`:

### 1. **Chat Block** (`Chat.tsx`)
**Spec fields:**
- `temperature`: Number as string (0-2)
- `instructions`: System prompt/instructions
- `max_tokens`: Max tokens to generate (empty string for unlimited)
- `stop`: Array of stop sequences
- `messages_code`: JavaScript function returning messages array
- `functions_code`: JavaScript function returning OpenAI functions
- `presence_penalty`: Optional penalty parameter
- `frequency_penalty`: Optional penalty parameter
- `top_p`: Optional sampling parameter
- `logprobs`: Boolean for log probabilities
- `top_logprobs`: Number of top logprobs to return

**Config fields:**
- `provider_id`: LLM provider (e.g., "openai", "anthropic")
- `model_id`: Model identifier (e.g., "gpt-4", "claude-3")
- `function_call`: Function calling mode
- `use_cache`: Boolean for caching
- `response_format`: Optional JSON schema for structured output

### 2. **Code Block** (`Code.tsx`)
**Spec fields:**
- `code`: JavaScript function code

**Config fields:**
- None (empty object)

### 3. **LLM Block** (`LLM.tsx`)
**Spec fields:**
- `temperature`: Number as string
- `max_tokens`: Max tokens to generate
- `stop`: Array of stop sequences
- `prompt`: Prompt template with {{variables}}
- `few_shot_preprompt`: Optional few-shot examples prefix
- `few_shot_count`: Number of few-shot examples
- `few_shot_prompt`: Few-shot prompt template
- `frequency_penalty`: Optional
- `presence_penalty`: Optional
- `top_p`: Optional
- `top_logprobs`: Optional

**Config fields:**
- `provider_id`: LLM provider
- `model_id`: Model identifier
- `use_cache`: Boolean for caching

### 4. **Data Source Block** (`DataSource.tsx`)
**Spec fields:**
- `query`: Search query or template
- `full_text`: Boolean for full text retrieval
- `filter_code`: JavaScript function for filtering

**Config fields:**
- `data_sources`: Array of data source configurations
- `top_k`: Number of results to retrieve
- `filter`: Filter object with tags and timestamp
- `use_cache`: Boolean for caching

### 5. **Input Block** (`Input.tsx`)
**Spec fields:**
- None (empty object)

**Config fields:**
- `dataset`: Dataset name to use as input

### 6. **Data Block** (`Data.tsx`)
**Spec fields:**
- `dataset`: Dataset identifier

**Config fields:**
- None (empty object)

### 7. **Search Block** (`Search.tsx`)
**Spec fields:**
- `query`: Search query
- `num`: Number of results (optional)

**Config fields:**
- `provider_id`: Search provider
- `use_cache`: Boolean for caching

### 8. **Curl Block** (`Curl.tsx`)
**Spec fields:**
- `scheme`: "http" or "https"
- `method`: HTTP method (GET, POST, etc.)
- `url`: URL endpoint
- `headers_code`: JavaScript function returning headers object
- `body_code`: JavaScript function returning body string

**Config fields:**
- `use_cache`: Boolean for caching

### 9. **Browser Block** (`Browser.tsx`)
**Spec fields:**
- `url`: URL to fetch
- `selector`: CSS selector for content extraction
- `timeout`: Timeout in milliseconds
- `wait_until`: Wait condition ("load", "networkidle2", etc.)
- `wait_for`: Optional selector to wait for

**Config fields:**
- `provider_id`: Browser provider
- `use_cache`: Boolean for caching
- `error_as_output`: Boolean to return errors as output

### 10. **Database Block** (`Database.tsx`)
**Spec fields:**
- `query`: SQL query

**Config fields:**
- `workspace_id`: Workspace ID for database
- `database_id`: Database identifier

### 11. **Database Schema Block** (`DatabaseSchema.tsx`)
**Spec fields:**
- None (empty object)

**Config fields:**
- `workspace_id`: Workspace ID
- `database_id`: Database identifier

### 12. **Map Block** (`MapReduce.tsx`)
**Spec fields:**
- `from`: Source block name to iterate over
- `repeat`: Optional repeat specification

**Config fields:**
- None (empty object)

### 13. **Reduce Block** (`MapReduce.tsx`)
**Spec fields:**
- None (empty object)

**Config fields:**
- None (empty object)

### 14. **While Block** (`WhileEnd.tsx`)
**Spec fields:**
- `condition_code`: JavaScript function returning boolean
- `max_iterations`: Maximum number of iterations

**Config fields:**
- None (empty object)

### 15. **End Block** (`WhileEnd.tsx`)
**Spec fields:**
- None (empty object)

**Config fields:**
- None (empty object)

## Key Patterns

1. **Spec vs Config Separation:**
   - `spec`: Contains the logic/content (prompts, code, queries)
   - `config`: Contains infrastructure choices (providers, models, caching)

2. **Common Config Fields:**
   - `use_cache`: Almost all blocks support caching
   - `provider_id`: Blocks that need external services
   - `model_id`: LLM blocks specifically

3. **JavaScript Code Fields:**
   - Several blocks use `_fun = (env) => {...}` pattern
   - Access to `env.state.BLOCK_NAME` for inter-block data flow

4. **Control Flow:**
   - Map/Reduce pairs for iteration
   - While/End pairs for conditional loops
   - Both use matching names and indent levels

5. **Data Flow:**
   - Input block provides initial data
   - Each block can access previous blocks via `env.state`
   - Code blocks act as transformers/processors
