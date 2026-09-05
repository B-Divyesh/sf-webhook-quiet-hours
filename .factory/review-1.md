# Review 1 — Group webhook failures before Slack

- **Final verdict: PASS**
- Findings: **0**
- Untested public claims: **0**
- Implementation reviewed: `eb8b5e072ce0f47d70b3d954c5c104330ac14465`
- Documentation baseline reviewed: `c794ef8a6e3c243c64c0064acb101804a6c7f2dd`
- Live build identity: `10eab6c88a98c3844a7c2da19aec77d26de48839`
- Live URL: `https://webhook-quiet-hours.sociobot.in`
- Reviewed: 2026-09-05 UTC

## Scope and identity

The product groups repeated internal webhook failures so small engineering
teams can act on urgent failures without sending every repeat to Slack.

Fresh phone (390×844) and desktop (1440×900) browsers showed the job, audience,
and first action before scrolling:

- Job: **Group webhook failures before they reach Slack**.
- Audience: small engineering teams that need urgent failures without every
  repeat going to Slack.
- First action: **Try it with sample data**.

`GET /health` returned HTTP 200 with live build SHA `10eab6c…`. A clean build at
the current documentation baseline produced HTML, JavaScript, and CSS with the
exact live SHA-256 hashes. `git diff eb8b5e0..c794ef8` contains only README and
factory documentation changes; product source is unchanged. The implementation
candidate is therefore `eb8b5e0`, while `10eab6c` is the documentation commit
reported by the deployed image. The later `c794ef8` commit is report-only.

## Demo and recovery

Fresh phone and desktop contexts completed the same isolated flow without an
account:

1. The first-screen action opened `/demo` in one click.
2. The populated sample showed 18 deliveries, three fingerprints, 15 compressed
   repeats, and three realistic ledger rows.
3. **Demo — sample data, nothing is saved** remained visible through detail
   inspection, acknowledgement, and reload.
4. Acknowledgement persisted after reload. **Reset demo** restored the high
   item. **Start for real** returned home and removed both demo session keys.

Only `demo:webhook-quiet-hours:*` keys existed during the flow. No admin or real
receiver key appeared, all observed demo requests stayed on the product origin,
and no console or page error occurred. A forced first provisioning failure
showed **Sample workspace did not open** and **Retry sample data**; the second
attempt recovered without an uncaught error. No real receiver data, customer
record, notification destination, or payment was created or changed.

## Checkout and license validation

The registered offer is present and matches the public copy:

- Live checkout returned HTTP 303 to `checkout.dodopayments.com`, followed by
  HTTP 200. The clean checkout displayed **Webhook Quiet Hours Field Station**,
  **$39.00**, and USD.
- Test checkout returned HTTP 303 to `test.checkout.dodopayments.com`, followed
  by HTTP 200 with the same product and price.
- Both actual billing verification endpoints returned HTTP 200 with
  `valid: false`, `reason: invalid`, and no expiry for a harmless invalid token.
- The live restore form displayed **License no longer active. Free features
  remain available.** Paid controls stayed locked.
- The valid return-token, URL stripping, local daily cache, revoked-license,
  license-header, and server-enforced paid-limit paths passed their declared
  recorded-verdict tests. Free export and acknowledgement remain available
  without a license.

This checks the public checkout, actual rejection path, client reconciliation,
and backend entitlement boundary. It does not rely on the redirect alone. No
real purchase was attempted.

## Declared claims

All 21 exact commands from `.factory/claims.json` were run independently after
`npm ci`; all passed.

| Claim | Result |
| --- | --- |
| demo-sandbox | Pass |
| demo-expiry | Pass |
| repeat-compression | Pass |
| csv-export | Pass |
| privacy-same-origin | Pass |
| one-time-price | Pass |
| license-validation | Pass |
| paid-limits | Pass |
| encrypted-payloads | Pass |
| encrypted-configuration | Pass |
| retention-cleanup | Pass |
| signed-ingress | Pass |
| ingress-body-limit | Pass |
| ingress-rate-limit | Pass |
| notification-policy | Pass |
| quiet-window | Pass |
| responsive-keyboard | Pass |
| api-rate-limit | Pass |
| zero-config-start | Pass |
| durable-data-path | Pass |
| graceful-shutdown | Pass |

The landing page, app states, legal pages, demo documentation, and README were
cross-checked against the inventory. Every public product promise maps to a
declared command; there are no unlisted or untested claims.

## Functional and backend checks

The full clean gate passed:

```text
npm ci
npm audit --audit-level=high
npm run check
npm run build
cargo build --release --locked
npm test
```

`npm audit` reported zero vulnerabilities. `npm test` passed 3 Vitest tests, 17
Rust unit/router tests, 3 process-level runtime tests, and 20 Playwright tests.
The build produced `dist/`: 35.62 KB JavaScript (11.76 KB gzip), 18.87 KB CSS
(5.07 KB gzip), and 2.22 KB HTML (0.75 KB gzip).

The backend tests cover empty, normal, invalid, boundary, and recovery paths:
HMAC acceptance and rejection; grouping; encrypted payloads, signing secrets,
and notification URLs; alias and retention license boundaries; 256 KiB ingress;
quiet windows; all three notification policies with actual local POST capture;
cleanup; CSV; demo expiry; zero-config startup; two-process SQLite persistence
under the configured data directory; and SIGTERM shutdown.

A fresh live burst from one forwarded address returned 40 HTTP 401 responses
and then 20 HTTP 429 responses. The 429 responses carried `Retry-After: 18`.
A different forwarded address remained independent, and `/health` stayed 200.
Health is intentionally exempt. The ingress allowance is proven by the exact
local claim command because creating a live alias would change real data.

## Accessibility, routes, privacy, offline, and performance

- The factory URL verifier passed in 561 ms with the correct title, `lang=en`,
  one `h1`, one `main`, complete image alternatives, labelled buttons, and no
  console or page errors.
- `/`, `/demo`, `/privacy`, `/terms`, and a deliberate unknown path were checked
  in light and dark themes at phone and desktop sizes. Each had one `h1`, one
  `main`, no horizontal overflow, and zero serious or critical axe findings.
  Titles were route-specific. The unknown path correctly returned HTTP 404 and
  a designed recovery page; that expected status is not a defect.
- At 390 px with 200% root text, the populated demo remained 390 px wide.
  Arrow-key tabs, route heading focus, polite route announcement, and back-button
  focus passed. The paid link and persistent demo controls measured at least
  44 px. Reduced-motion mode used non-smooth scrolling.
- The versioned service worker controlled the page and rendered the landing
  screen after an offline reload. Its cache included the built shell assets.
- First-load and demo traffic used no analytics, CDN font, third-party script,
  or cross-tenant telemetry. Billing traffic occurred only after the explicit
  restore/checkout checks.
- CSP, `frame-ancestors 'none'`, `nosniff`, `DENY` framing, and `no-referrer`
  were response headers. HTML and the service worker are `no-cache`; hashed
  JavaScript and CSS are immutable for one year. `robots.txt` and `sitemap.xml`
  returned their correct content types.
- Lighthouse mobile: Performance 100, Accessibility 100, Best Practices 100,
  SEO 100; LCP 1,080 ms, total blocking time 0 ms, CLS 0.

## Earlier finding disposition

Every finding from verifications 1–5 was inspected and re-proved:

| Earlier finding | Current evidence |
| --- | --- |
| Live build identity mismatch | Closed: live SHA is documented, source delta is documentation-only, and built assets match live exactly. |
| Missing immutable asset caching | Closed: hashed JS/CSS send one-year immutable caching. |
| Production required secrets | Closed: only-`PORT` startup generates, persists, and reuses secrets. |
| Paid-panel contrast | Closed: axe passes the populated Aliases panel and all live routes in both themes and sizes. |
| Tab arrow keys | Closed: ArrowLeft, ArrowRight, Home, and End regression coverage passes; live ArrowRight passed. |
| Lost status announcements | Closed: the stable live-region regression passes for every reported action. |
| Small persistent and legal targets | Closed: all regressions pass; live paid/demo targets are at least 44 px. |
| Missing claims inventory | Closed: 21 claims exist and all exact commands pass independently. |
| Missing demo and unclear first screen | Closed: job, audience, first action, isolation, reset, and exit pass live on phone and desktop. |
| Pre-auth traffic bypassed limiting | Closed: live burst is exactly 40 then 429 with `Retry-After`. |
| Discovery metadata and 404 absent | Closed: discovery files, metadata, route titles, and designed HTTP 404 pass. |
| Fixed/incomplete service-worker cache | Closed: versioned built assets and offline reload pass. |
| State outside `/data` | Closed: runtime mount-path and two-process persistence tests pass. |
| Checkout unavailable | Closed: live and test redirects, final HTTP 200 pages, product, and price pass. |
| Public allowance exceeded across replicas | Closed: one-replica allowance is observed live at exactly 40. |
| Public claims omitted | Closed: inventory expanded; the future-release promise was removed. |
| Demo failure stuck on loading | Closed: named error, retry, and clean recovery pass live. |
| 200% text overflow | Closed: populated live demo has no overflow. |
| Route focus/announcement absent | Closed: focus and polite announcement pass live. |

## Evidence

Evidence is under `/work/.evidence/webhook-quiet-hours-review-1/`, including
claim logs, full test/build logs, live browser JSON and screenshots, checkout
screenshots, HTTP/header captures, asset hashes, rate-limit results, URL verifier
output, and Lighthouse JSON.

**Final verdict: PASS — 0 findings and 0 untested claims.**
