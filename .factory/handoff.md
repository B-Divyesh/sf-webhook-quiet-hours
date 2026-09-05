# Webhook Quiet Hours — review 1 handoff

## Status

Independent review 1 passed with zero findings and zero untested claims. No
product code was changed. The implementation reviewed is
`eb8b5e072ce0f47d70b3d954c5c104330ac14465`; the repository documentation
baseline was `c794ef8a6e3c243c64c0064acb101804a6c7f2dd`. Live `/health` reports
documentation commit `10eab6c88a98c3844a7c2da19aec77d26de48839`.

The commits after `eb8b5e0` contain only README and factory documentation
changes. A clean build's HTML, JavaScript, and CSS hashes exactly match the live
assets, so no new product image is required for the report-only commits.

Full review: [.factory/review-1.md](review-1.md).

## What was verified

- Fresh phone and desktop browsers identified the job, audience, and
  **Try it with sample data** action before scrolling.
- The one-click demo showed 18 deliveries, 3 fingerprints, and 15 compressed
  repeats. Its sample label persisted through detail, acknowledgement, reload,
  and reset. Leaving removed the separate demo namespace.
- Forced demo provisioning failure recovered through the named retry action.
- All 21 declared claim commands passed independently. The complete `npm test`
  suite, checks, production build, release build, and dependency audit passed.
- Empty, invalid, boundary, recovery, notification, encryption, retention,
  startup, mounted-data persistence, and graceful-shutdown paths passed.
- A live API burst returned exactly 40 unauthorized responses and then 20
  rate-limit responses with `Retry-After`; a second client remained independent.
- Live and test checkout each returned 303 to the correct Dodo host and then
  HTTP 200. Both pages showed Webhook Quiet Hours Field Station for USD 39 once.
  Both real verify endpoints rejected an invalid token, and the UI remained on
  the free tier. Recorded-verdict tests cover valid, cached, revoked, and
  server-enforced entitlement paths.
- `/`, `/demo`, `/privacy`, `/terms`, and the designed HTTP 404 passed phone and
  desktop, light and dark axe checks with no serious or critical findings.
  Keyboard routing, focus, announcements, 200% text, 44 px targets, reduced
  motion, offline reload, privacy request boundaries, route titles, discovery
  files, and security/cache headers passed.
- Lighthouse mobile: Performance 100, Accessibility 100, Best Practices 100,
  SEO 100; LCP 1,080 ms, total blocking time 0 ms, CLS 0.

No real receiver data, notification destination, customer record, payment, or
license was created or changed.

## Run locally

```bash
npm ci
npm audit --audit-level=high
npm run check
npm run build
cargo build --release --locked
npm test
```

Run each public claim with the exact command in `.factory/claims.json`. The
sample entry point is `https://webhook-quiet-hours.sociobot.in/demo`.

## Evidence

Review evidence is in `/work/.evidence/webhook-quiet-hours-review-1/`. The
factory-facing copies are `/work/.evidence/qa-report.md` and
`/work/.evidence/qa-result.json`.

## Known gaps

No product finding remains. A real paid purchase was intentionally not made;
checkout availability, the displayed offer, invalid live validation, client
locking, and server entitlement boundaries were verified without creating a
customer or payment.
