Title: Auth and User Management — WorkOS

Scope
- Primary SSO/session and organization management for the Front application.

Key Code
- JWT verification (JWKS): `front/lib/api/workos.ts` (verify and parse claims using WorkOS JWKS).
- Session cookies: `front/lib/api/workos/user.ts` (seal/unseal, refresh, cookie domain handling, region metadata).
- Client wrapper: `front/lib/api/workos/client.ts`; Orgs & memberships: `front/lib/api/workos/*`.
- Auth wrappers: `front/lib/api/auth_wrappers.ts` integrates with WorkOS sessions.

Environment
- `WORKOS_API_KEY`, `WORKOS_CLIENT_ID`, `WORKOS_COOKIE_PASSWORD`, `WORKOS_ISSUER_URL`.
- Webhooks: `WORKOS_WEBHOOK_SECRET`, `WORKOS_WEBHOOK_SIGNING_SECRET`.
- Session cookie domain: `WORKOS_SESSION_COOKIE_DOMAIN` (prod), `WORKOS_ENVIRONMENT_ID`.

Operational Notes
- Legacy Auth0 mentions exist (for migration paths) but WorkOS is authoritative.
- Region metadata saved to user via WorkOS user management to help route requests in multi-region.

