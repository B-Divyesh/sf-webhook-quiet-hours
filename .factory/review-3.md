# Review 3 — Group webhook failures before Slack

- **Final verdict: PASS**
- Findings: **0**
- Untested public claims: **0**
- Implementation reviewed: `eb8b5e072ce0f47d70b3d954c5c104330ac14465`
- Documentation baseline reviewed: `ca60401301eb0c8fd52f009649bc33b20638c463`
- Live build identity: `10eab6c88a98c3844a7c2da19aec77d26de48839`
- Live URL: `https://webhook-quiet-hours.sociobot.in`
- Reviewed: 2026-09-06 UTC

## Job, audience, and first action

Webhook Quiet Hours groups repeated internal webhook failures so small
engineering teams can act on urgent failures without sending every repeat to
Slack. Before scrolling, fresh 390×844 phone and 1440×900 desktop browsers
both showed:

- Job: **Group webhook failures before they reach Slack**.
- Audience: small engineering teams that need urgent failures without sending
  every repeat into Slack.
- First action: **Try it with sample data**.

The action was inside the first viewport at both sizes and opened `/demo` in
one click. Both first screens started at scroll position zero with one `h1`,
one `main`, `lang="en"`, no overflow, and no console or page errors.

`/health` returned HTTP 200 with the live build SHA above. The repository diff
from `eb8b5e0` through `ca60401` contains only README and factory documentation;
there is no later product-code change. A fresh build produced HTML, JavaScript,
CSS, and service-worker hashes identical to the deployed files. The live build
identity is therefore a documentation build of the implementation reviewed.

## Demo, recovery, and real-data safety

Fresh phone and desktop demo workspaces immediately showed 18 deliveries,
three fingerprints, and 15 compressed repeats. **Demo — sample data, nothing
is saved** remained present through acknowledgement, reload, and reset.
Acknowledgement changed the high count from one to zero and survived reload;
**Reset demo** restored the high item and all three rows. **Start for real**
cleared both `demo:webhook-quiet-hours:*` session keys, removed the banner,
returned home, and deleted the temporary server workspace, which then returned
404.

All observed demo traffic stayed on the product origin and carried no admin
authorization. No real-data key was created. A forced first provisioning
failure displayed **Sample workspace did not open** and **Retry sample data**;
the retry restored all 18 deliveries without an uncaught error. No real
receiver, notification destination, purchase, customer, or license was
created or changed.

## Paid offer and license validation

The registered Field Station offer is intact in both modes:

- Live billing returned HTTP 303 to `checkout.dodopayments.com`, then HTTP 200.
- Test billing returned HTTP 303 to `test.checkout.dodopayments.com`, then
  HTTP 200.
- Both final pages showed **Webhook Quiet Hours Field Station**, **$39.00**,
  and a one-time license description.

Both real verification endpoints returned HTTP 200 with `valid: false` and
`reason: "invalid"` for a harmless test token. In a fresh live return-URL flow,
the product stored the token locally, removed it from the address bar, called
the live verifier, recorded the invalid verdict, kept paid features locked,
and said that free features remain available. This is actual verification, not
redirect-only evidence.

The declared `license-validation` test separately proves a recorded valid
verdict, once-daily caching, license forwarding for a paid action, and revoked
license reconciliation. `paid-limits` proves that the receiver returns 402 for
a second alias or longer retention without a valid license and allows both
with one. Free acknowledgement and CSV export were exercised without a
license.

## Claims and quality gates

All 21 exact commands in `.factory/claims.json` were run independently after
`npm ci`; every command passed:

| Claims | Result |
| --- | --- |
| demo-sandbox, demo-expiry, repeat-compression, csv-export | Pass |
| privacy-same-origin, one-time-price, license-validation, paid-limits | Pass |
| encrypted-payloads, encrypted-configuration, retention-cleanup | Pass |
| signed-ingress, ingress-body-limit, ingress-rate-limit | Pass |
| notification-policy, quiet-window, responsive-keyboard | Pass |
| api-rate-limit, zero-config-start, durable-data-path, graceful-shutdown | Pass |

Landing, demo, privacy, terms, README, demo documentation, and paid copy were
cross-checked against the inventory. Each public functional, privacy, price,
limit, security, retention, and runtime promise is covered; there are no
unlisted or untested public claims. The brief does not need a model-assisted
step: deterministic signature checking and fingerprint rules are the job, and
CSV export already covers the obvious portability need.

The complete clean gate passed:

```text
npm ci
npm audit --audit-level=high
npm run check
npm run build
cargo build --release --locked
npm test
```

`npm audit` reported zero vulnerabilities. `npm test` passed 3 Vitest tests,
17 Rust unit/router tests, 3 runtime process tests, and 20 Playwright tests.
The build produced `dist/` with 35.62 kB JavaScript (11.76 kB gzip) and
18.87 kB CSS (5.07 kB gzip). The 820 px hero is 62.43 kB.

## Browser, accessibility, and site structure

- `/`, `/demo`, `/privacy`, `/terms`, and an unknown path were checked in
  light and dark themes on phone and desktop. Normal routes returned 200; the
  designed unknown page deliberately returned 404. Each had its own title,
  one `h1`, one `main`, no overflow, and zero serious/critical axe findings.
- The expected browser message for the deliberate 404 document was not treated
  as a defect. The root URL verifier separately passed with zero load errors,
  correct title and language, labelled buttons, and complete image alternatives.
- At 200% root text, the 390 px populated demo had no horizontal overflow.
  ArrowRight moved from Observations to Aliases. Privacy navigation focused its
  `h1` and announced “Privacy loaded”; Back restored focus to the demo `h1`.
- Demo and paid controls measured at least 44 px. Reduced-motion mode used
  `scroll-behavior: auto`. A versioned service-worker cache included hashed
  assets and rendered the landing page after a fresh offline reload.
- Every discovered link and discovery asset resolved as intended. The paid
  link returned its expected 303; only links on the deliberate 404 retained
  that page's expected 404 response.
- First load requested only the product origin. There are no analytics,
  third-party scripts, or CDN fonts. CSP, `nosniff`, `DENY` framing, and
  `no-referrer` were present; HTML was `no-cache`, and hashed JS/CSS were
  immutable for one year.

Fresh mobile Lighthouse scores were Performance 100, Accessibility 100, Best
Practices 100, and SEO 100. LCP was 1,147 ms, total blocking time 51 ms, and
CLS 0.

## Backend and boundary paths

A disposable production binary started with only its normal runtime inputs,
generated secrets without logging their values, and returned the candidate
SHA from health. Invalid authentication and HMAC returned 401; an invalid
alias and invalid quiet time returned 400. Two correctly signed repeats
returned 202, grouped into one fingerprint, and reported one compressed repeat.
A corrected settings request returned 204. After SIGTERM and restart on the
same SQLite directory, the alias, two events, one fingerprint, and aggregate
counts were unchanged. The exact runtime claims also proved only-`PORT`
startup, persisted generated secrets, shared data-path operation across two
rolling processes, and clean shutdown.

On the public service, 60 simultaneous unauthenticated dashboard requests from
one forwarded client produced exactly 40 HTTP 401 responses and 20 HTTP 429
responses. The 429 carried `Retry-After: 19`; a different forwarded client
remained independent and `/health` stayed 200. Ingress burst/refill behavior,
the exact 256 KiB body boundary, demo tenant isolation, notification policy,
and encrypted SQLite retention are covered by direct router or process tests.

## Earlier finding disposition

All findings from verifications 1–5 and the later pass reports were inspected
and re-proved:

| Earlier category | Current evidence |
| --- | --- |
| Build identity and immutable caching | Source delta is documentation-only; clean local and live file hashes match; hashed assets are immutable. |
| Only-`PORT` startup and durable state | Runtime claims and the disposable restart flow passed with generated and persisted secrets. |
| Paid-panel contrast, tabs, status, and touch targets | Full Playwright suite plus live phone/desktop axe, keyboard, and size checks passed. |
| Missing claims, demo, and first-screen clarity | 21 exact claim commands passed; live one-click demo, reset, exit, and recovery passed. |
| Pre-auth and multi-replica allowance | Live burst was exactly 40 before 429 with `Retry-After`; another client remained independent. |
| Metadata, discovery, 404, and service worker | Route audit, link crawl, URL verifier, designed 404, cache version, and offline reload passed. |
| Checkout and entitlement | Both Dodo modes reached the correct $39 page; real invalid and tested valid/revoked enforcement passed. |
| Demo failure, 200% reflow, route focus, and legal targets | Live regressions and the complete Playwright suite passed. |

## Evidence

Evidence is in `/work/.evidence/webhook-quiet-hours-review-3/`, including each
claim log, complete gate logs, browser transcripts and screenshots, checkout
screenshots, rate-limit results, runtime restart results, route axe results,
URL verifier output, live/local hashes, and Lighthouse JSON.

**Final verdict: PASS — 0 findings and 0 untested claims.**
