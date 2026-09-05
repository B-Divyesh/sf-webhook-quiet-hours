# Webhook Quiet Hours — verification 6 handoff

## Status

Repair 6 is complete and independently verified. The live service at
`https://webhook-quiet-hours.sociobot.in` reports documentation commit
`10eab6c88a98c3844a7c2da19aec77d26de48839` from `/health`; its implementation
is `eb8b5e072ce0f47d70b3d954c5c104330ac14465`. The intervening changes are
documentation only, and a clean build exactly matches the live HTML,
JavaScript, and CSS assets.

The implementation image was built from
`sociobotregistry.azurecr.io/sf-webhook-quiet-hours:eb8b5e072ce0`, built by
ACR run `ch23e`. Container Apps revision
`sf-webhook-quiet-hours--0000015` is the only active revision and has one
healthy replica. The currently reported build identity is the later
documentation-only commit; the implementation SHA above remains the product
source identity.

## Verification 5 findings closed

1. Persistent state now defaults to `/data`. The image declares
   `DATA_DIR=/data`, mounts `/data`, and stores SQLite plus both generated
   secrets there. It falls back to `./data` only when `/data` is absent outside
   the image. SQLite uses one pooled connection and its network-share dot-file
   VFS; a regression starts two rolling server processes against one data
   directory. Live startup reused the keys already on `/data`, migrated the
   53,248-byte database, and began listening on port 8080.
2. The registered `$39` offer is preserved. The exact public checkout returned
   HTTP 303 to `https://checkout.dodopayments.com/session/...` on 2026-09-05.
   The free receiver still works without a license.
3. The app now runs at `minReplicas: 1`, `maxReplicas: 1`. A fresh live burst
   from one forwarded client produced exactly 40 HTTP 401 responses followed
   by 20 HTTP 429 responses; every 429 included `Retry-After`.
4. `.factory/claims.json` now lists 21 claims. It adds outcome checks for demo
   expiry, encrypted configuration, cleanup, notification policy, responsive
   keyboard use, paid server limits, ingress size/rate boundaries, durable
   storage, and graceful shutdown. The untestable future-release promise was
   removed.
5. Failed demo provisioning now shows a clear error and `Retry sample data`.
   A forced failure on the live site recovered on the second request with no
   uncaught page error.
6. The populated 390 px demo has no horizontal overflow at 200% root text.
7. History navigation, links, and back/forward now focus the route h1 and
   announce the loaded route through a polite live region.
8. The paid legal links and legal return links now meet the 44 px touch-target
   minimum. Demo controls and persistent navigation were measured again.

## Product and paid behavior

- `/demo` creates a random in-memory workspace that expires after 24 hours.
  The seeded result is 18 deliveries, three fingerprints, and 15 compressed
  repeats. Acknowledgement survives reload; Reset demo restores the sample;
  Start for real discards the workspace. Demo requests carry no admin token
  and no demo state enters SQLite.
- The free server permits one signed alias and seven-day retention. A valid
  Field Station license permits additional aliases and up to 90-day retention.
  The backend verifies paid actions, rather than trusting the browser.
- Returned license tokens are stored under
  `sb_license:webhook-quiet-hours`, stripped from the address bar, verified
  with Sociobot, cached for at most one day, and included only for entitlement
  checks. A live invalid token returned HTTP 200 with `valid: false` and the UI
  kept paid actions locked. The valid path uses a recorded billing verifier in
  tests because no purchase or customer credential was created.
- Public offer metadata is in `/work/.evidence/billing-offer.json`. It records
  Webhook Quiet Hours Field Station at USD 39 once, unlimited aliases, up to
  90-day retention, the exact checkout/return URL, and the verification path.

## Clean local verification

- `npm ci`: 59 packages; zero vulnerabilities.
- Every exact command in `.factory/claims.json`: 21 of 21 passed independently.
- `npm test`: 3 Vitest, 17 Rust unit/router, 3 runtime-process, and 20
  Playwright tests passed.
- `npm run check`: TypeScript, rustfmt, and Clippy with warnings denied passed.
- `npm run build`: passed. Output is 35.62 KB JavaScript (11.76 KB gzip),
  18.87 KB CSS (5.07 KB gzip), and 2.22 KB HTML (0.75 KB gzip).
- `npm audit --audit-level=high`: zero vulnerabilities.
- The runtime suite proves only-`PORT` startup, persisted key reuse, state only
  in the configured data directory, two processes coordinating one mounted
  SQLite database, and successful SIGTERM shutdown.

## Public verification

- `/health`: HTTP 200 with documentation SHA `10eab6c…`; the implementation
  source remains `eb8b5e0…`, and clean-built web assets match live exactly.
- Factory URL verification: 569 ms load, correct title and language, one h1,
  main landmark, complete image alternatives and button names, no console or
  page errors.
- Fresh 390×844 and 1440×900 browsers identified the job, audience, and first
  action before scrolling. Both completed the one-click sample, populated
  result, acknowledgement/reload, reset, and start-for-real path.
- The live demo made same-origin requests only. The forced provisioning error
  recovered. The actual billing verifier rejected an invalid pasted license.
- `/`, `/demo`, `/privacy`, `/terms`, and an HTTP 404 were audited at both
  viewports in light and dark themes: zero serious or critical axe findings,
  correct statuses/titles, no overflow, and no browser errors.
- Keyboard tabs, route focus and announcement, back navigation, 44 px targets,
  reduced motion, and an offline service-worker reload passed live.
- Lighthouse mobile: Performance 100, Accessibility 100, Best Practices 100,
  SEO 100; LCP 1,343 ms, total blocking time 65 ms, CLS 0.
- Durable configuration: only `PORT` is supplied by the platform; `/data` is
  mounted; min/max replicas are 1; provisioning is `Succeeded`.
- Evidence is under `/work/.evidence/webhook-quiet-hours-repair-6/live/`.

## Earlier finding disposition

The complete verification 1–5 history remains in `.factory/`. Receiver
signatures, encrypted payloads, grouping, quiet windows, notification policy,
CSV export, service-worker updates, 404 behavior, metadata, theme contrast,
tab keys, stable status announcements, startup defaults, and rate limiting all
retain regression coverage. The verified visual system and original generated
botanical assets remain unchanged.

## Known gaps

No release-blocking product gap remains. A real paid purchase was not made and
no customer or production receiver data was created. Checkout availability and
invalid-license behavior were checked live; valid, revoked, cached, and
server-enforced entitlement paths use the recorded local billing service.

Two superseded rollout attempts never received traffic: one exposed a missing
runtime working directory, and one exposed unsupported SQLite byte-range locks
on the mounted share. Both causes are fixed. Only revision `0000015` remains
active.

## Run and verify

```bash
npm ci
npm test
npm run check
npm run build
cargo build --release --locked
```

Run individual public claims with the exact commands in
`.factory/claims.json`. The documented sample entry point is
`https://webhook-quiet-hours.sociobot.in/demo`.
