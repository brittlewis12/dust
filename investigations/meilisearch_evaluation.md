# Meilisearch as SearchStore Alternative: Comprehensive Evaluation

## Executive Summary

This document evaluates Meilisearch's suitability as an alternative SearchStore implementation for the Dust codebase. Based on comprehensive research of Meilisearch's current capabilities (latest stable versions ~v1.16-v1.18), **Meilisearch is NOT a suitable replacement for ElasticSearch as the SearchStore backend**.

**Key Findings:**
- **Critical Feature Gaps**: Missing advanced aggregations needed for statistics and tag counting
- **Limited Query Complexity**: Cannot replicate ElasticSearch's complex boolean query logic
- **Pagination Constraints**: Hard 1000-result limit vs ElasticSearch's flexible pagination
- **Text Analysis Differences**: FST-based prefix matching vs edge n-gram analysis
- **Migration Complexity**: Significant reimplementation required with reduced functionality

**Recommendation**: Continue with ElasticSearch. Consider Meilisearch only for new, simplified use cases that don't require the full SearchStore feature set.

## Meilisearch Feature Overview

### Current Version & Capabilities

**Latest Stable Version**: ~v1.16-v1.18 (sources show conflicting information)

**Core Features Available**:
- **Hybrid Search**: Combines full-text and semantic search with AI embeddings
- **Vector Search**: Built-in vector storage with multiple embedder options (OpenAI, Hugging Face, Ollama)
- **Multi-tenancy**: Tenant tokens for secure workspace isolation
- **Performance**: Sub-second response times, up to 4x faster indexing than competitors
- **Text Analysis**: FST-based tokenization with language-specific pipelines

**Key Limitations Identified**:
- No advanced aggregation functions (only count, min/max)
- No complex boolean query syntax
- Limited pagination (1000 result maximum)
- No edge n-gram analysis
- Simplified query model vs ElasticSearch's flexibility

## Method-by-Method Capability Mapping

### 1. search_nodes Method

**ElasticSearch Implementation**:
```rust
async fn search_nodes(
    query: String,
    filters: SearchFilter,
    sort: Vec<SearchSort>,
    limit: usize,
    offset: Option<String>
) -> Result<(Vec<CoreContentNode>, u64, bool, Option<String>, Option<SearchWarningCode>)>
```

**Current Features**:
- Multi-index querying (`data_sources_*` + `data_sources_nodes_*`)
- Complex boolean queries with boost factors (10.0x exact, 5.0x phrase prefix, 2.0x standard text)
- Edge n-gram analysis for autocomplete
- Search-after pagination with cursor
- Query clause limiting (1024 clause protection)

**Meilisearch Mapping**:

| Feature | Meilisearch Support | Implementation | Gap Assessment |
|---------|-------------------|----------------|----------------|
| Multi-field search | ✅ Supported | Use `searchableAttributes` | **Equivalent** |
| Boost factors | ✅ Supported | Filter scoring with weights | **Different approach** |
| Boolean logic | ⚠️ Limited | Filter arrays (AND/OR) | **Major limitation** |
| Prefix matching | ✅ Supported | Built-in FST-based | **Different tech, similar result** |
| Pagination | ❌ Limited | Hard 1000 limit | **Critical limitation** |
| Relevance scoring | ✅ Supported | Built-in scoring + custom | **Adequate** |

**Implementation Challenges**:
- **Boolean Query Building**: ElasticSearch's complex nested boolean queries cannot be replicated
- **Boost Factor Mapping**: Would need complete recalibration using Meilisearch's filter scoring
- **Pagination**: Cannot support large result sets beyond 1000 items
- **Query Clause Limiting**: No equivalent protection mechanism

### 2. get_data_source_stats Method

**ElasticSearch Implementation**:
```rust
async fn get_data_source_stats(
    data_source_ids: Vec<String>
) -> Result<(Vec<DataSourceESDocumentWithStats>, i64)>
```

**Current Features**:
- Terms aggregation for grouping by data_source_id
- Sum aggregation for text_size calculation
- Multi-level nested aggregations

**Meilisearch Mapping**:

| Feature | Meilisearch Support | Implementation | Gap Assessment |
|---------|-------------------|----------------|----------------|
| Terms aggregation | ❌ Not available | Would need client-side processing | **Major gap** |
| Sum aggregation | ❌ Not available | Would need client-side calculation | **Major gap** |
| Nested aggregations | ❌ Not available | Not possible | **Critical gap** |
| Statistics | ⚠️ Very limited | Only min/max for numerical facets | **Insufficient** |

**Implementation Challenges**:
- **No Aggregation Framework**: Meilisearch only supports document counting and basic min/max
- **Client-Side Processing**: Would require fetching all documents and computing statistics in application code
- **Performance Impact**: Significant performance degradation for large datasets

### 3. search_tags Method

**ElasticSearch Implementation**:
```rust
async fn search_tags(
    query: Option<String>,
    filter: SearchFilter,
    limit: usize
) -> Result<Vec<(String, u64, Vec<(String, u64)>)>>
```

**Current Features**:
- Terms aggregation for unique tag collection
- Nested aggregations (tags grouped by data source)
- Case-insensitive filtering with prefix matching
- Default 200 tag limit

**Meilisearch Mapping**:

| Feature | Meilisearch Support | Implementation | Gap Assessment |
|---------|-------------------|----------------|----------------|
| Tag enumeration | ⚠️ Limited | facetDistribution for counting | **Partial support** |
| Nested grouping | ❌ Not available | Not possible | **Major gap** |
| Prefix filtering | ✅ Supported | Built-in prefix search | **Adequate** |
| Count aggregation | ✅ Supported | facetDistribution | **Adequate** |

**Implementation Challenges**:
- **No Nested Aggregations**: Cannot group tags by data source as required
- **Simplified Faceting**: Only basic counting available, no complex grouping

### 4. Document Lifecycle (index_node/delete_node)

**ElasticSearch Implementation**:
```rust
async fn index_node(&self, node: NodeItem) -> Result<()>
async fn delete_node(&self, node: NodeItem) -> Result<()>
```

**Meilisearch Mapping**:

| Feature | Meilisearch Support | Implementation | Gap Assessment |
|---------|-------------------|----------------|----------------|
| Document indexing | ✅ Supported | POST /indexes/{uid}/documents | **Equivalent** |
| Document deletion | ✅ Supported | DELETE /indexes/{uid}/documents/{id} | **Equivalent** |
| Batch operations | ✅ Supported | Batch document APIs | **Equivalent** |
| Timeout control | ✅ Supported | HTTP client configuration | **Equivalent** |

**Implementation Assessment**: ✅ **Fully compatible** - This is the strongest area of compatibility

### 5. Data Source Operations (index_data_source/delete_data_source)

**ElasticSearch Implementation**:
```rust
async fn index_data_source(&self, data_source: &DataSource) -> Result<()>
async fn delete_data_source(&self, data_source: &DataSource) -> Result<()>
```

**Meilisearch Mapping**:

| Feature | Meilisearch Support | Implementation | Gap Assessment |
|---------|-------------------|----------------|----------------|
| Document indexing | ✅ Supported | Standard document operations | **Equivalent** |
| Bulk deletion | ⚠️ Different | Filter-based deletion | **Different approach** |
| Transactional ops | ❌ Limited | No ACID transactions | **Potential consistency issues** |

**Implementation Challenges**:
- **Bulk Deletion**: Meilisearch uses filter-based deletion vs ElasticSearch's delete-by-query
- **Atomicity**: No transaction guarantees for multi-step operations

## Key Compatibility Analysis

### Text Analysis & Tokenization

**ElasticSearch Approach**:
- Edge n-gram analysis (1-20 characters)
- Custom analyzers: `edge_analyzer`, `tag_edge_analyzer`, `icu_analyzer`
- ICU tokenization for international text
- Word delimiter preservation

**Meilisearch Approach**:
- Finite State Transducers (FSTs) for prefix matching
- Language-specific tokenization pipelines
- Built-in typo tolerance
- No edge n-gram equivalent

**Compatibility Assessment**: ⚠️ **Different but functional**
- FSTs achieve similar autocomplete functionality
- May require adjustment of user expectations
- No fine-grained control over n-gram generation

### Filtering & Query Logic

**ElasticSearch Capabilities**:
```json
{
  "bool": {
    "should": [...],
    "must": [...],
    "filter": [...],
    "boost": 2.0
  }
}
```

**Meilisearch Capabilities**:
```json
{
  "filter": [
    ["category = 'electronics'", "price > 100"],
    "brand = 'apple'"
  ]
}
```

**Compatibility Assessment**: ❌ **Major limitations**
- Cannot replicate complex nested boolean queries
- Limited to simple AND/OR filter combinations
- No query-time boosting within boolean context

### Multi-tenancy & Security

**ElasticSearch Approach**:
- Index-level isolation using workspace-specific indices
- Filter-based permission enforcement
- Query-time security through must clauses

**Meilisearch Approach**:
- Tenant tokens with JWTs for secure access
- Filter rules embedded in tokens
- Single index with filtered access

**Compatibility Assessment**: ✅ **Strong alternative approach**
- Meilisearch's tenant tokens are more elegant
- Built-in security vs query-time filtering
- Better performance with single index architecture

### Performance & Pagination

**ElasticSearch Performance**:
- Search-after cursor pagination
- Unlimited result sets (with performance trade-offs)
- Configurable refresh intervals
- Complex query optimization

**Meilisearch Performance**:
- Sub-50ms response times
- Hard 1000 result limit (`maxTotalHits`)
- Offset-based pagination (performance degrades with large offsets)
- Optimized for user-facing search

**Compatibility Assessment**: ⚠️ **Performance gains with significant limitations**
- Faster for typical search use cases
- Cannot handle large result set requirements
- Pagination model doesn't match current architecture

## Feature Gap Analysis

### Critical Gaps (Migration Blockers)

1. **Advanced Aggregations**
   - **Missing**: Terms aggregation, sum aggregation, nested aggregations
   - **Impact**: Cannot implement `get_data_source_stats` or `search_tags` properly
   - **Workaround**: None viable - would require client-side processing

2. **Complex Boolean Queries**
   - **Missing**: Nested boolean query logic with arbitrary complexity
   - **Impact**: Cannot replicate current search behavior
   - **Workaround**: Simplified filtering only

3. **Unlimited Result Sets**
   - **Missing**: Pagination beyond 1000 results
   - **Impact**: Cannot serve large data exploration use cases
   - **Workaround**: None - fundamental architectural limitation

### Moderate Gaps (Require Changes)

1. **Edge N-gram Analysis**
   - **Alternative**: FST-based prefix matching
   - **Impact**: Different autocomplete behavior
   - **Workaround**: Acceptable for most use cases

2. **Query Complexity Management**
   - **Missing**: Clause limit protection and warnings
   - **Impact**: Less robust query handling
   - **Workaround**: Application-level query simplification

3. **Multi-index Architecture**
   - **Alternative**: Single index with type differentiation
   - **Impact**: Different data modeling approach
   - **Workaround**: Consolidate to single index design

### Minor Gaps (Cosmetic Changes)

1. **Boost Factor Precision**
   - **Alternative**: Filter scoring system
   - **Impact**: Requires recalibration
   - **Workaround**: Re-tune relevance weights

2. **Error Handling Specificity**
   - **Alternative**: Different error response format
   - **Impact**: Update error handling code
   - **Workaround**: Straightforward adaptation

## Migration Complexity Assessment

### Code Changes Required

**SearchStore Implementation** (Estimated effort: **16-20 weeks**)
- Complete rewrite of all query logic
- New aggregation handling with client-side processing
- Different pagination model implementation
- Updated error handling and response parsing

**Data Model Changes** (Estimated effort: **4-6 weeks**)
- Consolidate to single index architecture
- Update field mapping and analysis configuration
- Revise permission filtering approach

**Application Integration** (Estimated effort: **8-12 weeks**)
- Update all search UI components for 1000-result limit
- Revise aggregation displays (statistics, tag clouds)
- Implement new pagination controls
- Performance testing and optimization

### Data Migration

**Index Restructuring**:
- Merge `data_sources_*` and `data_sources_nodes_*` indices
- Update document schemas for single-index model
- Migrate permission filtering to tenant tokens

**Content Analysis**:
- Rebuild search indices with FST-based analysis
- Test autocomplete behavior changes
- Validate relevance scoring

### Risk Assessment

**High Risks**:
- **Feature Parity Loss**: Critical aggregation features cannot be replicated
- **Performance Degradation**: Client-side aggregation processing for large datasets
- **User Experience Impact**: 1000-result limit affects power users

**Medium Risks**:
- **Development Timeline**: 6+ month project with significant complexity
- **Testing Requirements**: Extensive validation of changed search behavior
- **Rollback Complexity**: Difficult to revert due to architectural changes

**Low Risks**:
- **Security Model**: Meilisearch's tenant tokens are superior
- **Basic Search**: Core document search functionality is equivalent

## Implementation Recommendations

### If Proceeding with Meilisearch (NOT Recommended)

1. **Phase 1: Proof of Concept** (4-6 weeks)
   - Implement basic document indexing and search
   - Test tenant token security model
   - Validate performance characteristics

2. **Phase 2: Feature Assessment** (6-8 weeks)
   - Build client-side aggregation system
   - Test with production data volumes
   - Measure performance impact of limitations

3. **Phase 3: Full Implementation** (12-16 weeks)
   - Complete SearchStore rewrite
   - Update all dependent systems
   - Extensive testing and optimization

### Alternative Recommendations

1. **Continue with ElasticSearch**
   - Maintain current feature parity
   - Consider OpenSearch as alternative
   - Investigate ElasticSearch performance optimizations

2. **Hybrid Approach**
   - Use Meilisearch for simple search interfaces
   - Keep ElasticSearch for complex aggregations
   - Maintain dual backends (increased complexity)

3. **Future Evaluation**
   - Monitor Meilisearch roadmap for aggregation features
   - Reconsider when feature gaps are addressed
   - Evaluate for new, simpler use cases

## Conclusion

**Meilisearch is not a suitable replacement for ElasticSearch** as the SearchStore backend due to fundamental feature gaps that cannot be adequately addressed:

1. **Critical Missing Features**: The absence of advanced aggregations makes it impossible to implement key SearchStore methods (`get_data_source_stats`, `search_tags`) without significant performance penalties.

2. **Architectural Limitations**: The 1000-result pagination limit and simplified query model are incompatible with the current SearchStore design and user expectations.

3. **Migration Complexity**: The required changes are so extensive that it would essentially be building a new search system with reduced capabilities.

**However**, Meilisearch excels in areas where it's designed to be used:
- **User-facing search interfaces** with simple, fast autocomplete
- **Content discovery** applications with moderate result set sizes
- **Multi-tenant SaaS** applications requiring secure search isolation

**Recommendation**: Maintain ElasticSearch for the SearchStore implementation. Consider Meilisearch for future, purpose-built search interfaces that don't require the full complexity of the current SearchStore feature set.

---

*This evaluation is based on Meilisearch capabilities as of August 2025 and the Dust SearchStore requirements documented in `/Users/britt/code/dust/investigations/searchstore_elasticsearch_analysis.md`*