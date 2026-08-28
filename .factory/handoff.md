# Webhook Quiet Hours — build handoff

## Repair 2 — release-blocking verification remediation (2026-08-28 UTC)

This repair addresses every finding in `.factory/verification-1.md` while
preserving the existing Rust/Axum + Vite container artifact and product
behavior.

- **Deployment identity:** the runtime image accepts a `BUILD_SHA` Docker build
  argument and retains it as its runtime environment value. Release images are
  built with the exact committed source revision; the deployment configuration
  also sets that same value explicitly. `GET /health` is the acceptance check
  and must return the deployed revision verbatim.
- **Cache policy:** static routing now applies
  `Cache-Control: public, max-age=31536000, immutable` only to Vite-style
  fingerprinted files under `/assets/`. Public non-fingerprinted images use a
  one-day cache, while the SPA shell, deep links, and `/sw.js` use `no-cache`
  so PWA updates remain discoverable. The server regression test exercises all
  four cases.

### Repair verification

- Clean install: `npm ci` passed (0 vulnerabilities).
- Unit/integration: `npm test` passed — 3 Vitest tests and 6 Rust tests,
  including `static_files_have_update_safe_cache_headers`.
- Type, format and lint: `npm run check` passed (strict TypeScript, rustfmt,
  Clippy with warnings denied).
- Production artifacts: `npm run build` passed (26.23 kB raw / 9.07 kB gzip
  JS; 15.69 kB raw / 4.50 kB gzip CSS) and
  `cargo build --release --locked` passed.
- Production-configured release binary on port 18080: `/health` returned the
  injected test identity; root, SPA deep link, service worker and static asset
  cache headers matched the policies above. CSP, `nosniff`, DENY framing and
  `no-referrer` remained present.
- Browser smoke at 1440×900 and 390×844: zero page/console errors, one h1,
  main landmark, no horizontal overflow, visible keyboard focus, reduced-motion
  transform disabled, and no third-party landing-page requests. `/privacy` and
  `/terms` both rendered.
- Accessibility: factory `verify-url.sh` passed; Playwright axe at 390 px
  reported zero violations (including zero serious/critical).
- Offline/update: after service-worker control, a 390 px offline reload still
  rendered the main content.
- Container package verification is performed by the configured ACR build and
  the public `/health` identity check during this repair deployment. Docker and
  Podman are not installed in this worker.

## Independent verification 1 — 2026-08-28 UTC

**FAIL — candidate `d854693319e5f9cf993dff39a51f56ca82d4a8e3` is not
verified as deployed.** Fresh public `/health` at
`https://webhook-quiet-hours.sociobot.in` returned build SHA
`742c55ba4df05cb6fac46a5a6761c54448b6502f`, not the candidate. Although
the candidate only changes handoff documentation and its product source equals
the older SHA, the required deployment identity check fails. Do not promote
until an image identifying as `d854693…` is deployed and rechecked.

The verifier passed clean install, tests, type/lint checks, Vite production
build, locked Rust release build, production-configured local end-to-end
ingress/aggregation/security/concurrency checks, responsive browser checks,
axe, reduced motion, offline reload, and Lighthouse. Public hashed assets lack
`Cache-Control` immutable caching, a medium-severity follow-up. Full evidence,
exact commands, and all defects are in `.factory/verification-1.md`.

## Shipped

Webhook Quiet Hours is a complete single-team, self-hosted v1. Rust/axum serves
the built Vite/TypeScript interface and JSON API from one container on port 8080;
SQLite is the only state dependency.

- Authenticated endpoint-alias creation with an unguessable URL key and optional
  HMAC-SHA256 verification (`X-Hub-Signature-256` and
  `X-Webhook-Signature`). Secrets are shown once.
- AES-256-GCM encryption at rest for retained raw payloads, HMAC secrets, and the
  notification destination. Production startup requires an admin token and a
  32-byte base64 encryption key.
- Stable event fingerprints derived from endpoint, event type, top-level shape,
  and status/error; repeat counts are compressed without losing the latest
  encrypted sample.
- Normal digest, immediate high severity, and record-only rules. High severity
  bypasses quiet hours, resets acknowledgement on a new observation, records
  overdue targets, and sends immediately when a rule is promoted.
- Quiet-window/UTC-offset settings, digest cadence, Slack-compatible outgoing
  webhook, a single review/runbook link, manual digest, delivery error state,
  configurable payload deletion, CSV export, and global receiver rate limiting.
- Responsive 390 px dashboard with setup, loading, empty, delivery-error,
  offline, and success states; native dialogs, destructive confirmation,
  clipboard affordances, visible focus, reduced motion, light/dark treatments,
  and keyboard-operable controls.
- $39 one-time Field Station browser unlock using the Sociobot checkout,
  returned-token storage, daily verification cache, optimistic offline cached
  verdict, revocation handling, and paste-to-restore. It expands aliases and
  retention; signing, escalation, and export stay free. The server is open-source
  self-hosted software, so this is intentionally an honor-system UI license.
- `/privacy` and `/terms`, a strict CSP and security headers, no analytics or
  third-party runtime assets, and a cache-versioned service worker.
- Multi-stage non-root Docker image, SQLite volume, structured JSON logs,
  graceful shutdown, health/build-SHA route, migrations, README, and MIT license.

The botanical field-guide visual system and asset provenance are in
`.factory/design.md`. The accepted source and prompt are in `assets/src/`; the
shipped 480/820 px WebPs are 24/61 KB.

## Run and deploy

Development:

```sh
npm ci
npm run build
ADMIN_TOKEN=local-dev-token cargo run
```

Production container:

```sh
docker build -t webhook-quiet-hours .
docker run --rm -p 8080:8080 \
  -e ADMIN_TOKEN='<long random value>' \
  -e DATA_ENCRYPTION_KEY='<base64 32-byte value>' \
  -e PUBLIC_URL='https://webhook-quiet-hours.sociobot.in' \
  -v webhook-quiet-hours-data:/app/data \
  webhook-quiet-hours
```

The deploy target is the container on `PORT=8080`; persistent data is
`/app/data`. `GET /health` returns status and `BUILD_SHA`.

## Verification completed

- `npm test`: pass — 3 frontend tests and 5 Rust tests, including a real-router
  create-alias → HMAC-sign → receive → fingerprint integration test.
- `npm run check`: pass — strict TypeScript, rustfmt, and Clippy with warnings as
  errors.
- `npm run build`: pass — output at `dist/index.html`; initial JS 26.23 KB raw
  (9.07 KB gzip), CSS 15.69 KB raw (4.50 KB gzip), mobile hero 24 KB.
- `cargo build --release --locked`: pass.
- Production-mode release binary: pass — `/`, `/privacy`, `/terms`, and
  `/health` return HTTP 200 with required production configuration.
- Worker `verify-url.sh`: pass — title, `lang=en`, exactly one `h1`, main
  landmark, image alt, labelled buttons, and zero page/console errors.
- Playwright 390×844 and desktop end-to-end smoke: pass — authenticated login,
  alias creation, webhook HTTP 202, fingerprint appearance, decrypted detail;
  zero horizontal overflow and zero console errors.
- Axe browser audit: zero violations on landing, authenticated dashboard,
  privacy, and terms at 390 px.
- Lighthouse mobile: Performance 100, Accessibility 100, Best Practices 100,
  SEO 100; LCP 1.2 s, total blocking time 20 ms, CLS 0.
- Load smoke: 100 concurrent `/health` requests, 0 failures in 183 ms
  (approximately 546 requests/second on the worker).

Browser and Lighthouse evidence was generated locally under the ignored
`.factory/evidence/` directory.

## Repair delivery QA — 2026-08-28 UTC

**Product-QA result: PASS.** Candidate
`742c55ba4df05cb6fac46a5a6761c54448b6502f` was recovered without product
source changes, visual changes, or a deployment-class change. The existing
immutable ACR image
`sociobotregistry.azurecr.io/sf-webhook-quiet-hours:742c55ba4df0` was reused;
the repaired worker path registered the hostname before issuing and binding the
managed certificate.

- Clean dependency install: `npm ci` completed with 0 vulnerabilities.
- Frontend and backend test gate: `npm test` passed — 3 Vitest assertions and
  5 Rust tests; 0 failures.
- Static analysis gate: `npm run check` passed — strict TypeScript, rustfmt,
  and Clippy with `-D warnings`.
- Build gate: `npm run build` passed (26.23 kB raw / 9.07 kB gzip initial JS;
  15.69 kB raw / 4.50 kB gzip CSS), and
  `cargo build --release --locked` passed.
- Container readiness: the deployed revision served the candidate image on
  port 8080 with production-only configuration supplied as platform secrets.
- Public acceptance: `GET https://webhook-quiet-hours.sociobot.in/` returned
  `HTTP/2 200`; `GET https://webhook-quiet-hours.sociobot.in/health` returned
  `HTTP/2 200` and
  `{"build_sha":"742c55ba4df05cb6fac46a5a6761c54448b6502f","status":"ok"}`.
- Public browser smoke: worker `verify-url.sh` passed against the public root
  in 641 ms with zero page/console errors; title present, `lang=en`, exactly
  one `h1`, a `main` landmark, no images missing alt text, and no unlabelled
  buttons.

Repair evidence is retained under the ignored
`.factory/evidence/repair-1/` directory.

## Known gaps and next steps

- The repair worker reused the already-successful immutable image rather than
  rebuilding it; the deployed container itself was then verified on both its
  platform FQDN and public custom domain.
- No real Slack-compatible destination was contacted, avoiding an external side
  effect. Delivery uses a bounded four-second HTTPS POST and exposes the last
  failure in the dashboard; connect a test incoming webhook and use “Send digest
  now” during deployment acceptance.
- The factory still needs to register the test/live Sociobot paid product. No
  product ID is hardcoded; staging can build with
  `VITE_BILLING_BASE=https://pilot-api.sociobot.in` before the release build uses
  the production default.
- Provider-specific canonical signature formats beyond GitHub/generic raw-body
  HMAC-SHA256 are not inferred. Create unsigned private aliases only for trusted
  internal senders or add a provider adapter before accepting a different
  signature scheme.
