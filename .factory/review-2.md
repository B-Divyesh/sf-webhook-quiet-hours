# Review 2 — Group webhook failures before Slack

- **Final verdict: PASS**
- Findings: **0**
- Untested public claims: **0**
- Implementation reviewed: `eb8b5e072ce0f47d70b3d954c5c104330ac14465`
- Documentation baseline reviewed: `48d8295e7f8898e7fd7d073fd9e6419bcaac6b6b`
- Live build identity: `10eab6c88a98c3844a7c2da19aec77d26de48839`
- Live URL: `https://webhook-quiet-hours.sociobot.in`
- Reviewed: 2026-09-06 UTC

## Scope and identity

Webhook Quiet Hours groups repeated internal webhook failures so small
engineering teams can act on urgent failures without sending every repeat to
Slack. Before scrolling, fresh 390×844 phone and 1440×900 desktop browsers
both showed:

- Job: **Group webhook failures before they reach Slack**.
- Audience: small engineering teams that need urgent failures without sending
  every repeat into Slack.
- First action: **Try it with sample data**.

Both pages loaded at scroll position zero with one `h1`, one `main`,
`lang="en"`, no overflow, and no page or console errors. `/health` returned
HTTP 200 and the live build SHA above. The source implementation remains
`eb8b5e0`; the changes through the documentation baseline affect factory docs,
claims inventory, and README only, not `src/`, `frontend/`, Docker, or
migrations. The deployed `10eab6c` is likewise a documentation/report build.

## Demo, recovery, and privacy

The first action opened `/demo` in one click on both fresh devices. The sample
was populated with three realistic fingerprint rows from 18 deliveries and 15
compressed repeats. **Demo — sample data, nothing is saved** stayed visible
through acknowledgement, reload, and reset. Acknowledgement remained at zero
high items after reload; **Reset demo** restored the high item to one; **Start
for real** returned home and removed the two `demo:webhook-quiet-hours:*`
session keys. Only product-origin requests occurred during the complete demo
flow, with no real-data keys present.

The declared recovery regression forces a failed demo provision, verifies the
named **Sample workspace did not open** state and **Retry sample data** action,
then verifies a successful retry without an uncaught error. It passed in the
independent claim run.

## Paid offer and license validation

The registered Field Station offer is intact. Both public checkout endpoints
returned HTTP 303 to their correct Dodo host (live and test respectively), and
a fresh browser followed each to HTTP 200 checkout pages showing **Webhook
Quiet Hours Field Station** and **$39**. No purchase, customer, or real
receiver data was created.

This was not treated as redirect-only evidence. Both live billing verification
endpoints returned HTTP 200 for a harmless invalid token with
`valid: false`, `reason: "invalid"`, and no expiry. The independent
`license-validation` claim verifies URL token capture and stripping, local
daily cache, valid-license paid settings request header, and a later revoked
verdict locking paid retention. `paid-limits` verifies receiver enforcement:
the free tier rejects a second alias and longer retention with 402, while the
recorded valid verifier permits them. Free acknowledgement and CSV export are
exercised without a license.

## Claims and quality gates

All 21 exact commands in `.factory/claims.json` were run independently after
`npm ci`; every command passed. This includes demo isolation/expiry, grouping,
CSV, same-origin privacy, price and license behavior, paid limits, encryption,
retention, signed ingress, both limiters, notification policy, quiet window,
keyboard/reflow, zero-config startup, durable data, and SIGTERM shutdown.
There are no public promises on the landing page, legal pages, demo
documentation, or README without a declared tested claim.

The clean quality gates also passed:

```text
npm audit --audit-level=high     # 0 vulnerabilities
npm run check
npm run build                    # dist/ produced
cargo build --release --locked
npm test                         # 3 Vitest, 17 Rust unit/router,
                                 # 3 runtime, and 20 Playwright tests passed
```

The production build contains 35.62 kB JavaScript (11.76 kB gzip) and 18.87 kB
CSS (5.07 kB gzip). Fresh live Lighthouse mobile scored Performance 99,
Accessibility 100, Best Practices 100, and SEO 100; LCP was 1,163 ms, total
blocking time 136 ms, and CLS 0.

## Live runtime checks

- `/`, `/demo`, `/privacy`, `/terms`, and an unknown path were checked in
  light and dark treatments on phone and desktop. Each normal route returned
  200 with a route-specific title; the designed unknown page deliberately
  returned 404. All had one `h1`, one `main`, no horizontal overflow, and zero
  serious or critical axe findings.
- `/opt/fleet/lib/verify-url.sh` passed against the live URL: correct title,
  language, main landmark, image alternatives, labelled buttons, and no
  console or page errors.
- A fresh service-worker-controlled context rendered the landing screen after
  offline reload. Reduced-motion mode reported `scroll-behavior: auto`.
- Live header checks found `nosniff`, `DENY` framing, `no-referrer`, CSP with
  `frame-ancestors 'none'`, and no cache for the HTML shell. The app has no
  analytics, third-party script, or CDN font.
- A same-client 60-request unauthenticated dashboard burst returned exactly
  40 HTTP 401 responses and 20 HTTP 429 responses; the 429 carried
  `Retry-After: 17`. A different forwarded client received 401 and `/health`
  stayed 200.

## Earlier finding disposition

All findings in verifications 1–5 and review 1 were rechecked and remain
closed:

| Earlier category | Current evidence |
| --- | --- |
| Build identity and immutable caching | Health identity is documented; implementation source is unchanged; clean build and current headers pass. |
| Only-`PORT` startup and durable state | Exact zero-config and `/data` claim commands passed, including two-process persistence and restart reuse. |
| Paid-panel contrast, tabs, status, touch targets | Full Playwright suite and phone/desktop axe checks passed. |
| Missing claims, demo, and first-screen clarity | 21 tested claims, one-click isolated demo, reset/exit, clear job/audience/action, and recovery test all passed. |
| Pre-auth and multi-replica allowance | Current live burst is exactly 40 before 429 with `Retry-After`; independent-client check passed. |
| Metadata, discovery, 404, service worker | Current route/axe audit, designed 404, offline reload, and URL verifier passed. |
| Checkout and entitlement | Both final Dodo checkouts are HTTP 200 at $39; actual invalid validation and tested valid/revoked enforcement passed. |
| Demo failure, 200% reflow, route focus, legal targets | Exact regressions passed in the full suite and route audit found no overflow. |

## Evidence

Evidence is in `/work/.evidence/webhook-quiet-hours-review-2/`: individual
claim logs, complete gate logs, live phone/desktop/demo and checkout
screenshots, rate-limit counts, URL verifier output, and Lighthouse JSON.

**Final verdict: PASS — 0 findings and 0 untested claims.**
