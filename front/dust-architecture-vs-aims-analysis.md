# The Fundamental Architecture vs. Aims Mismatch

## The Core Problem: What Dust Is vs. What We Need It To Be

### What Dust Currently Is
A **SaaS platform** where:
- Apps are database records owned by workspaces
- Execution happens through a web UI with session auth
- The "product" is the hosted service, not the apps themselves
- Apps are intentionally not code (the "JSX moment" philosophy)
- Everything flows through workspace/user permissions
- The platform controls all execution and state

### What We Need for Self-Hosting and CLI Development
A **programmable AI execution engine** where:
- Apps are portable artifacts you can version, share, and deploy
- Execution can happen anywhere (CLI, CI/CD, edge, self-hosted)
- Apps are the product - composable, reusable, testable
- Developers can build and iterate without a web UI
- Authentication is optional or pluggable
- The engine is embeddable in different contexts

## The Architectural Mismatches

### 1. **Apps as Data vs. Apps as Code**

**Current:** Apps live in PostgreSQL as JSON blobs tied to workspace IDs
**Need:** Apps as files/packages that can be:
- Checked into git
- Diffed and reviewed
- Deployed independently
- Composed into larger systems
- Tested in isolation
- Published to registries

**Why This Matters:** You can't build a ecosystem of reusable AI components when each app is trapped in a database row. The assistant-v2-multi-actions-agent should be a published package, not workspace ID 1's private data.

### 2. **Distributed State vs. Portable Execution**

**Current:**
- Core (Rust) owns execution state
- Front (Node) owns app definitions
- No single source of truth
- Creating an app requires coordinating two services
- Orphan states when coordination fails

**Need:**
- Single artifact that contains everything needed to run
- Stateless execution engine
- State externalized to pluggable stores
- Atomic operations

**Why This Matters:** Self-hosting means running on different infrastructures. Requiring PostgreSQL + Redis + Core + Front + perfect coordination makes it nearly impossible.

### 3. **Web-First vs. API-First**

**Current:**
- Session cookies for auth
- Browser-based flows
- API is secondary (often broken or incomplete)
- No service accounts or API keys for core operations
- Mixed auth models (session vs. API key)

**Need:**
- CLI-first development
- Programmatic access to everything
- Service accounts for automation
- Consistent auth model
- Local development with no auth

**Why This Matters:** Building the assistant-v2 requires rapid iteration. Having to click through a UI for each test run kills productivity.

### 4. **Workspace-Centric vs. Component-Centric**

**Current:**
- Everything belongs to a workspace
- Apps can't exist without workspace context
- No sharing between workspaces
- No public registry or marketplace
- Can't compose apps across workspaces

**Need:**
- Apps as standalone components
- Composition across boundaries
- Public and private registries
- Dependency management
- Version resolution

**Why This Matters:** The power of software comes from composition. If every workspace rebuilds assistant-v2 from scratch, we've failed.

### 5. **Monolithic Platform vs. Modular Engine**

**Current:**
- Must run entire Dust stack
- Can't run just the execution engine
- Can't swap out components
- Tied to specific databases and services
- No clear API boundaries

**Need:**
- Minimal execution runtime
- Pluggable providers
- Swappable storage backends
- Clear interface definitions
- Embeddable in other systems

**Why This Matters:** Self-hosting scenarios vary wildly. Enterprise wants on-prem. Startups want Vercel. Researchers want Colab. One size fits none.

## The Real Impact on Our Goals

### Goal: Build assistant-v2-multi-actions-agent
**Blocked by:**
- Can't develop iteratively without UI
- Can't version control progress
- Can't test components in isolation
- Can't share work-in-progress
- Can't rollback mistakes
- Can't compose from smaller tested pieces

### Goal: Enable Self-Hosting
**Blocked by:**
- Requires entire Dust infrastructure
- No clear deployment artifacts
- Complex distributed state management
- Workspace model doesn't translate
- No migration path for data

### Goal: Build a Developer Ecosystem
**Blocked by:**
- No way to share apps
- No package management
- No composition model
- No testing framework
- No local development story
- No CI/CD integration

## What Would Need to Change

### 1. **Apps as Packages**
- File-based app definitions
- Package manifest (dust.yaml)
- Dependency management
- Version control friendly
- Published to registry

### 2. **Execution Engine as Library**
- `dust-runtime` as npm package
- Pluggable providers
- Pluggable storage
- No database required for execution
- State passed in, not assumed

### 3. **CLI as First-Class**
- `dust new app`
- `dust run app.dust`
- `dust test`
- `dust publish`
- `dust install assistant-v2`

### 4. **Clear Separation of Concerns**
- Runtime (executes Dust apps)
- Registry (stores and serves packages)
- Platform (provides UI, auth, billing)
- Storage (pluggable state backends)

### 5. **Progressive Enhancement**
- Apps run locally with no auth
- Add auth when needed
- Add persistence when needed
- Add UI when needed
- Add collaboration when needed

## The Bottom Line

The current architecture is optimized for **Dust as a SaaS product** where the company controls everything and users interact through a web UI.

What we need is **Dust as a platform/engine** where developers can build, compose, and deploy AI apps in any context.

These aren't just different features - they're fundamentally different architectures. The friction we're experiencing isn't from bad implementation; it's from trying to use a SaaS product architecture as a development platform.

The "apps as data" philosophy sounds innovative, but it's actually the core problem. Real platforms (Node, Python, Rust) treat apps as code for good reasons: portability, composability, version control, testing, distribution. By rejecting these patterns, Dust makes basic developer workflows unnecessarily complex or impossible.
