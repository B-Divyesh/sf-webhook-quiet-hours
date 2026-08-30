# Webhook Quiet Hours — build handoff

## Repair 5 — independent-verification-4 release blockers (2026-08-30 UTC)

**Product-QA result: PASS.** The Rust/Axum + SQLite/Vite single-container
artifact and every receiver behavior that verification 4 passed are preserved.

### Findings, root causes, and repairs

1. `.factory/claims.json` now declares ten visitor-relevant claims with an
   exact standalone test command for each. Every command passed from the demo
   entry point or a temporary backend database/process, as appropriate.
2. `/demo` now provisions a random, 24-hour sample workspace with 18 realistic
   deliveries grouped into three fingerprints. Its mutable state is carried in
   the separate `demo:webhook-quiet-hours:state` session namespace, so it stays
   isolated from SQLite and consistent across container replicas. The banner,
   reset, start-for-real, notification suppression, and documentation are all
   implemented. A live multi-connection probe caught and removed the original
   single-replica memory assumption before final packaging.
3. The first screen now says “Group webhook failures before they reach Slack,”
   names small engineering teams, exposes the one-click demo as its primary
   action, explains the server-token path, and gives privacy, storage, and price
   facts. The required information order and paid section are present.
4. The API governor now wraps authentication. A fixed forwarded IP receives 40
   unauthorized responses followed by 429 responses with `Retry-After`; a new
   IP remains independent. This is covered by the exact Rust regression and was
   repeated against the public ingress.
5. The inline `$39` purchase link is an inline-flex 44 px target. The mobile
   browser test measures it alongside both persistent demo controls.
6. `robots.txt` and `sitemap.xml` are real typed files. Canonical, Open Graph,
   Twitter, 1200×630 social art, Apple touch icon, route titles, and a designed
   HTTP 404 route are shipped and crawled by Playwright.
7. Vite now injects its hashed JS/CSS into a content-versioned service-worker
   precache. Old caches are removed on activation, navigations use a network
   update with an offline fallback, and fingerprinted assets handle Vite’s
   base64url hash alphabet when applying one-year immutable caching.

Exact regression coverage is in `frontend/e2e/release-findings.spec.ts`,
`frontend/e2e/verifier-findings.spec.ts`, and the `src/lib.rs` test module.
`.factory/demo.md` documents the sandbox; `.factory/copy-audit.md` contains the
landing word counts and terminology audit.

### Clean local evidence

- `npm ci`: 59 packages installed; `npm audit --audit-level=high`: 0
  vulnerabilities.
- Every `.factory/claims.json` command passed independently.
- `npm test`: 3 Vitest tests, 10 Rust unit/router tests, the production
  startup/restart integration, and 17 Playwright tests all passed.
- `npm run check`: strict TypeScript, rustfmt, and Clippy with warnings denied
  passed. `npm run build` and `cargo build --release --locked` passed.
- Production output: 34.44 KB JS (11.42 KB gzip), 17.88 KB CSS (4.88 KB
  gzip), 62.43 KB largest hero WebP, and 38.41 KB 1200×630 social WebP.
- Factory `verify-url.sh` passed locally in 580 ms: correct title/lang, one h1,
  main, complete alt text and control labels, and no console/page errors.
- Desktop 1440×900 and mobile 390×844 passed light/dark axe serious/critical,
  keyboard demo entry, roving tabs, dialog focus return, reduced motion, 44 px
  targets, no overflow, demo reset/discard, CSV contents, same-origin privacy,
  service-worker update, and offline reload.
- Local mobile Lighthouse: Performance 100, Accessibility 100, Best Practices
  100, SEO 100; LCP 905 ms, TBT 0 ms, CLS 0.
- Local response policy: shell/SW `no-cache`, hashed JS/CSS one-year immutable,
  valid CSP/nosniff/DENY/no-referrer, real text/XML discovery responses, and
  unknown route HTTP 404. A 60-request unauthenticated API burst produced
  40×401 then 20×429; sampled `Retry-After` was 19 seconds.

### Container and public evidence

- Source repair commit `01bd1e999e15064c4087ce8a4793f10845542ff6`
  was pushed and built by ACR run `ch1cf`. Image
  `sociobotregistry.azurecr.io/sf-webhook-quiet-hours:01bd1e999e15` has digest
  `sha256:4db02d87a5204f33494fbcb5a5eea809f9c7f8d3e43f0026a296d09d048784a9`.
- Container Apps revision `sf-webhook-quiet-hours--0000011` reached
  `Succeeded` with only `PORT` configured. Public `/health` returned the exact
  source SHA above.
- Public `verify-url.sh` passed in 626 ms with no console/page errors. Root,
  `/demo`, discovery files, immutable assets, and the HTTP 404 route returned
  the intended status, type, cache, and security headers.
- Fresh public 390×844 and 1440×900 contexts on `/` and `/demo` had one h1,
  main, no overflow, no browser errors, no cross-origin request, and zero axe
  serious/critical findings. The public service worker completed an offline
  390 px reload.
- Public mobile Lighthouse: Performance 100, Accessibility 100, Best Practices
  100, SEO 100; LCP 1,052 ms, TBT 49 ms, CLS 0.
- A public 60-request unauthenticated burst produced 40×401 then 20×429;
  sampled `Retry-After` was 18 seconds.

### Known gaps and next steps

No release-blocking product gap remains. Docker is unavailable in this worker,
so the Dockerfile was packaged by the configured ACR cloud builder rather than
a local daemon. No real notification destination, billing purchase, license,
or production tenant data was created or changed. Independent re-verification
should use `/demo` and the commands in `.factory/claims.json`.

## Independent verification 4 — FAIL (2026-08-30 UTC)

Candidate `123678eb3a4b6341fc84d0c0eb018f1cb12e6cab` at
`https://webhook-quiet-hours.sociobot.in` **FAILS release verification**.

The deployed `/health` SHA and live HTML/JS/CSS hashes exactly match the candidate, so this is a candidate defect rather than a deployment-only failure. Core local tests, checks, production frontend build and Rust release build pass; receiver flow, encryption/signature grouping, CSV, validation and authenticated rate limiting were exercised. Full evidence is in `.factory/verification-4.md`.

Release blockers: `.factory/claims.json` is missing; the first screen lacks the mandated one-click sample-data demo and plain first-read explanation; `/demo` is only SPA fallback HTML; and unauthenticated live API traffic bypasses the API rate limiter (60 same-client requests all returned 401, never 429/`Retry-After`). Do not deploy/release until these are corrected and independently re-verified.

## Repair 4 — verifier accessibility and interaction remediation (2026-08-30 UTC)

**Product-QA and deployment result: PASS.** This repair preserves the
Rust/Axum + SQLite/Vite single-container artifact and all verified webhook
behavior. It supersedes the independent-verification-3 failure below.

### Exact reproduction and root causes

- The verifier's authenticated Aliases failure reproduced before the change on a
  390 px browser: axe reported the paid-card eyebrow at **4.19:1**
  (`#71865F` on `#172019`) and **Restore license** at **1.07:1**
  (`#F4F0E5` on `#FBF8EF`). The same source also reproduced at 1440 px.
- ArrowRight on the selected Observations tab left both focus and
  `aria-selected` unchanged because the tab buttons had no key handler or
  roving tabindex.
- A real 300 ms check after a mocked no-op manual digest found an empty
  `#live-status`: dashboard rendering replaced the live region immediately
  after `showStatus`.
- At 390 px the brand, Privacy, and Terms link boxes measured 23.31 px,
  21.06 px, and 21.06 px high.

### Repair and regression coverage

- The live region is now a stable `aria-atomic` sibling of the render root, so
  acknowledgement, classification, alias deletion, quiet-rule save, and
  manual-digest messages survive dashboard refreshes.
- Tabs now implement active roving tabindex plus ArrowLeft/ArrowRight,
  ArrowUp/ArrowDown, Home, and End. They expose `aria-controls` and the panel
  exposes the matching `aria-labelledby` relationship.
- The paid card supplies theme-specific AA eyebrow colors and an explicit
  ink-on-paper Restore control. The persistent brand and legal links have
  44×44 px minimum hit areas.
- `frontend/e2e/verifier-findings.spec.ts` is run by `npm test` with pinned
  Playwright 1.58.2 and axe-core. It audits the authenticated Aliases panel at
  390×844 and 1440×900 in light and dark themes, exercises roving tab keys,
  proves the same live-region node and message persist after every reported
  action, and measures all three mobile hit areas.
- The release audit also corrected two backend-service contract gaps: every
  authenticated API route now uses the first `X-Forwarded-For` hop for a
  20 req/s, burst-40 limiter with a `429` and non-zero `Retry-After`; the
  existing hook allowance remains 100 req/s, burst 200 per source IP. Rust's
  Docker builder is now `rust:1-slim` and accepts `BUILD_SHA=dev` locally.
  `api_rate_limit_uses_first_forwarded_hop_and_sets_retry_after` is the exact
  regression test.

### Local verification evidence

- Fresh `npm ci` installed 59 packages and `npm audit --audit-level=high`
  reported 0 vulnerabilities.
- `npm test`: passed — 3 Vitest tests, 7 Rust unit/router tests, the
  process-level production startup/restart integration, and 5 Playwright
  regressions.
- `npm run check`, `npm run build`, and `cargo build --release --locked` all
  passed. The production bundle is 26.59 KB raw / 9.22 KB gzip JavaScript and
  16.08 KB raw / 4.56 KB gzip CSS.
- A disposable real release binary with a SQLite database was authenticated,
  seeded with an alias, and audited at 390×844 and 1440×900. Axe returned no
  color-contrast violations, ArrowRight moved Aliases to Quiet rules, all
  persistent link targets were at least 44 px tall, and there were no page or
  console errors.
- `/opt/fleet/lib/verify-url.sh http://127.0.0.1:18080` passed: HTTP 200,
  title, `lang=en`, one h1, main landmark, complete image alt text, labelled
  controls, and no console/page errors (576 ms local load).

### Container deployment and public evidence

- Commit `37f2ea60abfba027d5598ead4cae75abfeace399` was pushed to `main` and
  deployed through factory ACR build run `ch1ab`. The successful image is
  `sociobotregistry.azurecr.io/sf-webhook-quiet-hours:37f2ea60abfb`, digest
  `sha256:1bf4512fac08645743a04684e8360d1dbbf636668810f332fffb7083f89f3d8b`.
- Container Apps revision `sf-webhook-quiet-hours--0000009` reached
  `Succeeded` with its only configured runtime environment name `PORT`.
- Public `GET /health` returned HTTP 200 and exact identity
  `{"build_sha":"37f2ea60abfba027d5598ead4cae75abfeace399","status":"ok"}`.
- Public `verify-url.sh` passed in 787 ms with no console/page errors, title,
  `lang=en`, exactly one h1, main, complete image alt text, and labelled
  controls. The public shell and `/sw.js` are `no-cache`; `nosniff`, DENY
  framing, no-referrer, and the self-only/billing-API CSP are present.
- A fresh 390 px public browser context received no third-party requests or
  browser errors. Its service worker controlled the page, completed
  `registration.update()`, and rendered the landing h1 after an offline
  reload. The authenticated color/keyboard/touch audit was run against the
  disposable real release binary because the public generated admin token is
  intentionally unavailable outside the container volume.

## Independent verification 3 — FAIL (2026-08-28 UTC)

**Latest acceptance result: FAIL. Do not accept or promote candidate
`c2baeb81b8e9b70385e8669a6e6143694debbb7b`.** The public `/health` identifies
that exact SHA, and live `index.html`, JS, CSS, images, and service worker match
the clean candidate build byte-for-byte, so this is not a deployment-identity
failure.

The release blocker is an axe `serious` color-contrast failure on the
authenticated Aliases purchase panel at both 390×844 and 1440×900. “Restore
license” renders at 1.07:1, making the only device-migration path nearly
unreadable, and the $39 eyebrow renders at 4.19:1; both require 4.5:1. Secondary
defects: the ARIA tablist does not respond to arrow keys, success/no-op live
status messages are destroyed by the immediate dashboard rerender, and the
home/Privacy/Terms links have 21–23 px mobile hit-box heights rather than 44 px.

Everything outside those UI findings passed fresh verification: clean install
and audit; all 10 tests; TypeScript/rustfmt/Clippy; Vite and locked Rust release
builds; only-`PORT` secret generation and restart persistence; signed ingress,
aggregation, encryption-at-rest searches, validation boundaries, local
notification delivery, CSV, 100-way concurrency, and process restart; exact
live identity/assets; public/legal/offline/reduced-motion/privacy checks; and
mobile Lighthouse 100/100/100/92 with 933 ms LCP, 52.5 ms TBT, and zero CLS.

Full commands, evidence, limitations, and remediation are in
`.factory/verification-3.md`. This latest independent result supersedes the
builder-authored Repair 3 PASS section below.

## Repair 3 — zero-configuration production startup (2026-08-28 UTC)

**Product-QA result: PASS.** The release blocker in
`.factory/verification-2.md` is repaired without changing the researched scope,
visual system, artifact class, API behavior, or deployment class.

### Finding, reproduction, and root-cause repair

- Before the repair, the verifier's production command
  `env -i APP_ENV=production PORT=18081 target/release/webhook-quiet-hours`
  exited 1 with `ADMIN_TOKEN is required in production`.
- `AppConfig::from_env` no longer substitutes a development credential or
  requires secret environment variables. When absent, it creates independent
  256-bit values with `OsRng`, stores them as `admin-token` and
  `encryption-key` beside the SQLite database, and reuses those files on later
  boots. Files are written via a same-directory temporary file, synced,
  atomically renamed, and mode `0600` on Unix.
- Valid `ADMIN_TOKEN` and `DATA_ENCRYPTION_KEY` values still override the
  persisted defaults. Startup logs a single structured line with only
  `generated`, `persisted`, or `supplied` source labels and the storage
  directory; secret values never enter logs.
- The dashboard and README now tell an operator where to retrieve the generated
  admin token. Container and local defaults remain `/app/data` and `data/`
  respectively.
- `tests/runtime_startup.rs` is the exact process-level regression: it clears
  the child environment, supplies only `PORT`, reaches `/health`, checks both
  0600 files, authenticates with the generated token, creates an encrypted
  signed alias, restarts in the Dockerfile's `APP_ENV=production` mode without
  either secret, submits a valid signed event, verifies the files are unchanged,
  and asserts generated/persisted log provenance without secret disclosure.

### Clean release evidence

- `npm ci`: passed; 55 packages installed, 0 vulnerabilities.
- `npm test`: passed; 3 Vitest tests, 6 Rust unit/router tests, and 1 binary
  startup/restart integration test.
- `npm run check`: passed; strict TypeScript, rustfmt, and Clippy with warnings
  denied.
- `npm run build`: passed and produced `dist/`; initial JS 26,265 bytes raw /
  9.08 KB gzip, CSS 15,691 bytes raw / 4.50 KB gzip, mobile hero 23,638 bytes.
- `cargo build --release --locked`: passed.
- Exact repaired-binary reproduction: fresh production boot returned
  `{"build_sha":"development","status":"ok"}`, created both secret files at
  mode 600, and logged generated/generated; restart returned healthy and logged
  persisted/persisted.
- Backend/response-policy smoke: unauthenticated summary 401, unknown
  authenticated API route 404, invalid alias 400, 262,144-byte ingress 202,
  262,145-byte ingress 400, encrypted marker absent from raw SQLite, CSV 200
  with `text/csv; charset=utf-8`, and 100 concurrent health requests passed.
  The shell, service worker, legal routes, and SPA fallback are `no-cache`;
  hashed assets are one-year immutable; public images are one-day cached. CSP,
  `nosniff`, DENY framing, and `no-referrer` remain present.
- Local browser QA at 1440×900 and 390×844 passed 39 assertions: one h1/main,
  no horizontal overflow, no public console/page errors or third-party
  requests, visible 3 px skip-link focus, keyboard-semantic tab access, dialog
  focus placement, alias creation and one-time secret UI, dark treatment,
  reduced motion, privacy/terms, service-worker control, and offline mobile
  reload. Axe reported zero violations on landing light/dark, dashboard, alias
  dialog, privacy, and terms.
- Local mobile Lighthouse: Performance 100, Accessibility 100, Best Practices
  100, SEO 92; LCP 1,277 ms, TBT 38 ms, CLS 0.

### Container and live acceptance

- The work-order deployer completed ACR build run `chbp` for
  `sociobotregistry.azurecr.io/sf-webhook-quiet-hours:0bc9d6f4a429`, digest
  `sha256:9033f86dd14856b256b490c7d8b395102e7dde66a991461148ec34081ea3cca0`.
  The Dockerfile built without `.git`, runs as the non-root `quiet-hours` user,
  and receives build identity through `BUILD_SHA`.
- Container Apps revision `sf-webhook-quiet-hours--0000007` reached Succeeded
  with exactly one configured runtime environment name, `PORT`. Its startup log
  reported generated/generated at `/app/data`, proving the public container did
  not depend on externally injected secrets.
- Public `/health` returned HTTP 200 and exact build identity
  `0bc9d6f4a429ff265cdc8f531e840ae3cb24f4b5`. Factory `verify-url.sh` returned
  200 with no console/page errors. Six live desktop/mobile page audits covering
  root, privacy, and terms had zero axe violations, no overflow, and no
  third-party requests; reduced motion and offline reload passed.
- Live mobile Lighthouse: Performance 100, Accessibility 100, Best Practices
  100, SEO 92; LCP 930 ms, TBT 7 ms, CLS 0.

Ignored reproducibility artifacts, JSON reports, logs, and screenshots are in
`.factory/evidence/repair-3/`. No real notification webhook, billing purchase,
or billing configuration was mutated during functional QA; deployment used the
factory-managed DNS and certificate path from the work order.

## Independent verification 2 — FAIL (2026-08-28 UTC)

**Candidate `a607aa5bd48d24a4b741db08e0438db76ece6469` fails the supplied
backend-service acceptance contract. Do not accept or promote it.** The public
deployment itself is healthy and correctly reports that exact SHA at
`https://webhook-quiet-hours.sociobot.in/health`; the preceding deployment-only
identity failure is resolved.

The blocking defect is production startup: `Dockerfile` sets `APP_ENV=production`
and `src/lib.rs` exits unless both `ADMIN_TOKEN` and `DATA_ENCRYPTION_KEY` are
provided. Fresh release-binary reproduction with only `PORT` returned
`Error: Config("ADMIN_TOKEN is required in production")`. The factory contract
requires startup with only `PORT`, CSPRNG generation plus persistent storage of
secret-like values when absent, and a non-secret generated-versus-supplied
startup log. None is implemented. The live deployment's externally injected
secrets do not satisfy that contract.

Everything else freshly exercised passed: clean install; 3 Vitest and 6 Rust
tests; TypeScript/rustfmt/Clippy; Vite production build; locked Rust release
build; local production-mode signed ingress, compression, encrypted payload
storage, validation/recovery, acknowledgement, CSV, size boundaries, and 100
concurrent requests; live desktop/390 px, keyboard focus, reduced motion,
axe, privacy/terms, PWA offline reload, security/cache headers, privacy request
audit, and Lighthouse (99 performance / 100 accessibility / 100 best practices
/ 92 SEO). Docker/Podman/Buildah were unavailable, so the exact Dockerfile
assembly could not be run; its Node/Vite and release-Rust stages were run.

Full commands, exact response evidence, limitations, and remediation are in
`.factory/verification-2.md`.

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
- Container package: configured ACR build succeeded for
  `sociobotregistry.azurecr.io/sf-webhook-quiet-hours:fa8724480cbf`
  (digest `sha256:3d8fdd606c8a8063c51dc25309578cf82c9808b31ed5837a3a09c55c1fcffa29`).
  The live Container Apps revision was healthy with the required admin-token
  and encryption-key secret references, `PUBLIC_URL`, and `BUILD_SHA` present.
- Public release acceptance: fresh `/health` returned HTTP 200 and
  `{"build_sha":"fa8724480cbfb58efb5101968f2f5e069de896d7","status":"ok"}`;
  public `/`, the hashed JS/CSS, the hero WebP, `/sw.js`, and `/privacy`
  returned exactly the cache policies described above. The public factory
  browser verifier completed in 603 ms with zero errors; live desktop and 390
  px keyboard/browser smoke plus 390 px axe reported no failures.
- Docker and Podman are not installed in this worker; the configured ACR build
  is the container package/consumer verification.

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
  notification destination. Missing credentials are generated with a CSPRNG and
  persisted beside SQLite; environment values remain optional overrides.
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
  -v webhook-quiet-hours-data:/app/data \
  webhook-quiet-hours
```

The deploy target is the container on `PORT=8080`; persistent data is
`/app/data`. Read the first-boot dashboard credential from
`/app/data/admin-token`. `GET /health` returns status and `BUILD_SHA`.

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
