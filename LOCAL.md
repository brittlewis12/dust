LOCAL.md
===

## Prereqs

- Node.js 20.13.0
- Rust (2021 edition toolchain)
- CMake 3 (required to build sentencepiece-sys; cmake@4 is incompatible with the pinned crate)

## Setup (updated for WorkOS auth + optional Temporal)

1) Create `.env.local` from the example and review values
    ```sh
    cp .env.local.example .env.local
    ```

   Notes:
   - This repo now uses WorkOS for auth locally. Ensure you set `WORKOS_*` vars as indicated in `.env.local.example` and register `http://localhost:3000/api/workos/callback` in the WorkOS dashboard.
   - Kibana requires `KIBANA_ENCRYPTION_KEY` (random 32+ chars) in env for the docker-compose service.
   - GCS emulation is optional for MVP. Core’s databases store can point to Postgres only; data source document storage can be added later.

2) Start infra services

    Run docker compose (Elasticsearch, Kibana, Postgres, Redis, Qdrant):

    ```sh
    docker compose --env-file .env.local up
    # add -d to run detached
    ```

3) Initialize Core DBs, Qdrant collections, and Elasticsearch indices

    With required services up, the init script needs environment variables from `.env.local`:

    **Using mise (recommended for mise users):**
    ```sh
    MISE_ENV_FILE='.env.local' mise x -- ./init_dev_container.sh
    ```

    **Using standard shell (export vars):**
    ```sh
    export $(grep -v '^#' .env.local | xargs) && ./init_dev_container.sh
    ```

    **Or source the file:**
    ```sh
    set -a && source .env.local && set +a && ./init_dev_container.sh
    ```

    After `init_dev_container.sh`, run these database setup commands (also need env vars):
    ```sh
    # Using mise:
    cd front && MISE_ENV_FILE='../.env.local' mise x -- ./admin/init_db.sh --unsafe
    cd front && MISE_ENV_FILE='../.env.local' mise x -- ./admin/init_plans.sh --unsafe
    cd connectors && MISE_ENV_FILE='../.env.local' mise x -- ./admin/init_db.sh --unsafe
    cd core && MISE_ENV_FILE='../.env.local' mise x -- cargo run --bin init_db
    ```
4) Build the client JS SDK
    ```sh
    cd sdks/js
    npm i --frozen-lockfile
    npm run build
    cd -
    ```
5) Install npm packages for connectors (optional for MVP)
    ```sh
    cd connectors
    npm i --frozen-lockfile
    ```
6) Run local database migrations for connectors (optional for MVP)
    ```sh
    DUST_REGION=local CONNECTORS_DATABASE_URI=postgres://dev:dev@localhost:5432/dust_connectors npm run initdb -- --unsafe
    > connectors@0.1.0 initdb
    > ./admin/init_db.sh --unsafe

    Running initdb
    {"level":"info","time":1744310165996,"pid":86039,"hostname":"[YOUR_HOST].local","msg":"Done"}
    ```
7) Run Core Rust API database migrations
    ```sh
    cd core
    DUST_REGION=local CORE_DATABASE_URI=postgres://dev:dev@localhost:5432/dust_api OAUTH_DATABASE_URI=postgres://dev:dev@localhost:5432/dust_oauth DATABASES_STORE_DATABASE_URI=postgres://dev:dev@localhost:5432/dust_databases_store cargo run --bin init_db
    ```
8) Install npm packages for the Front (Next.js) project
    ```sh
    cd front
    npm i --frozen-lockfile
    ```
9) Run local database migrations for Front
    ```sh
    DUST_REGION=local FRONT_DATABASE_URI=postgres://dev:dev@localhost:5432/dust_front npm run initdb
    ```
10) Initialize free_test and free_upgraded plans
    ```sh
    DUST_REGION=local FRONT_DATABASE_URI=postgres://dev:dev@localhost:5432/dust_front npx tsx ./admin/init_plans.ts
    Free plan FREE_TEST_PLAN created.
    Free plan FREE_UPGRADED_PLAN created.
    ```
11) Initialize your first workspace to generate workspace & space IDs
    ```sh
    DUST_REGION=local FRONT_DATABASE_URI=postgres://dev:dev@localhost:5432/dust_front npx tsx ./admin/init_dust_apps.ts --name [YOUR_WORKSPACE_NAME]
    Creating group
    Creating space
    export DUST_APPS_WORKSPACE_ID=[YOUR_WORKSPACE_ID]
    export DUST_APPS_SPACE_ID=[YOUR_SPACE_ID]
    ---
    - Restart front with the new env variables
    - Navigate to: http://localhost:3000/poke/[YOUR_WORKSPACE_ID]/spaces/[YOUR_SPACE_ID]
    - Run the "Sync dust-apps" plugin
    ```
    - With `DUST_APPS_WORKSPACE_ID` and `DUST_APPS_SPACE_ID` exported, start Front:
      ```sh
      # Using mise:
      cd front && MISE_ENV_FILE='../.env.local' mise x -- npm run dev
      # Or standard shell:
      cd front && npm run dev  # (after exporting env vars)
      ```
    - Navigate to `/poke/[WORKSPACE_ID]/spaces/[SPACE_ID]` and run "Sync dust-apps".

12) Login locally (WorkOS)

    - Ensure these envs are set: `WORKOS_API_KEY`, `WORKOS_CLIENT_ID`, `WORKOS_COOKIE_PASSWORD`, `WORKOS_ISSUER_URL`, `REGION_RESOLVER_SECRET`.
    - In WorkOS dashboard, add redirect URI: `http://localhost:3000/api/workos/callback`.
    - Visit `http://localhost:3000/api/workos/login` to sign in (or `?screenHint=sign-up`).

    To upgrade your workspace to the free unlimited plan:
    ```sh
    cd front && MISE_ENV_FILE='../.env.local' mise x -- npx tsx ./admin/cli.ts workspace upgrade --wId YOUR_WORKSPACE_ID
    ```

    Note: The workspace ID is shown in the URL after login (e.g., `7aVwrq0Dro`).

13) Optional: Temporal dev for long-running assistants

    Assistants run sync-first, with a 20s timeout fallback to Temporal. To avoid failovers for demos:

    ```sh
    temporal server start-dev
    export TEMPORAL_NAMESPACE=default
    export TEMPORAL_AGENT_NAMESPACE=default
    export TEMPORAL_CONNECTORS_NAMESPACE=default
    cd front && node start_worker.ts -w agent_loop update_workspace_usage
    ```

    You can also add npm scripts to automate this (see front/package.json after patch).
