# Verification 6 — Group webhook failures before Slack

- Verdict: **PASS**
- Findings: **0**
- Untested public claims: **0**
- Implementation reviewed: `eb8b5e072ce0f47d70b3d954c5c104330ac14465`
- Documentation reviewed: `10eab6c88a98c3844a7c2da19aec77d26de48839`
- Live URL: `https://webhook-quiet-hours.sociobot.in`
- Verified: 2026-09-05 UTC

## Scope and live identity

The product job is to group repeat webhook failures so a small engineering team
can act on urgent failures without routing every repeat to Slack. The first
screen states that job, names small engineering teams, and presents **Try it
with sample data** as the first action. It is visible before scrolling at
390×844 and 1440×900.

`GET /health` returned HTTP 200 and build SHA
`10eab6c88a98c3844a7c2da19aec77d26de48839`. This is the documentation commit.
The implementation under review is `eb8b5e0`; the difference from `eb8b5e0`
to `10eab6c` is documentation only (`README.md`, claims inventory, and factory
handoff). A clean build at `10eab6c` produced the exact live HTML, JavaScript,
and CSS hashes, and its frontend and backend source is unchanged from the
implementation candidate. This is therefore a documentation-identity update,
not a stale product image.

## Demo and public browser verification

Fresh phone and desktop browser contexts completed the same flow:

1. The visible first action opened `/demo` in one click.
2. The sample showed 18 deliveries, three fingerprints, and 15 compressed
   repeats.
3. **Demo — sample data, nothing is saved** stayed visible. Acknowledging the
   high-severity sample survived reload; **Reset demo** restored it; **Start
   for real** returned to the landing page.
4. Browser storage contained only `demo:webhook-quiet-hours:*` session keys
   during the demo and no real local-data keys. All observed demo requests were
   same-origin. No browser console or page error occurred.

The 390 px populated sample had no horizontal overflow at 200% root text.
Keyboard ArrowRight moved from Observations to Aliases; Privacy navigation and
Back both focused the destination `h1`, and the polite status read “Privacy
loaded”. Reduced-motion mode uses `scroll-behavior: auto`. A service-worker
controlled page rendered the landing screen after an offline reload.

The live `/`, `/demo`, `/privacy`, `/terms`, and deliberate unknown-path page
were tested at phone and desktop widths, in light and dark themes. Every page
had its expected status, route title, one `h1`, one `main`, no overflow, and
zero axe serious or critical violations. The unknown path deliberately returned
HTTP 404 with the designed recovery page; it is expected behavior, not a
finding. The Playwright axe integration was used because this repository does
not provide a `verify-url.sh` script.

## Backend, checkout, privacy, and performance

- Live unauthenticated dashboard burst from one forwarded address: 40 HTTP 401
  responses followed by 20 HTTP 429 responses. Sampled 429 responses included
  `Retry-After: 18`. A different forwarded address remained independent.
- `GET /api/v1/products/webhook-quiet-hours/checkout` with redirects disabled
  returned HTTP 303 to the Dodo hosted checkout. A harmless invalid license
  verification returned HTTP 200 with `valid: false`; it did not unlock any
  paid feature. No purchase or customer data was created.
- Clean local runtime tests passed with only `PORT`, generated/reused secrets,
  state under the configured durable-data path, two-process SQLite sharing,
  and graceful SIGTERM. Demo isolation is covered both by its claim test and
  by the live storage/network inspection.
- Live root headers include CSP with response-header `frame-ancestors 'none'`,
  `nosniff`, `DENY` framing, and `no-referrer`. Hashed CSS/JS responses are
  immutable for one year; HTML is `no-cache`. There were no first-load
  third-party requests, analytics, CDN fonts, or scripts.
- Lighthouse (mobile): Performance 100, Accessibility 100, Best Practices
  100, SEO 100; LCP 1,122 ms, total blocking time 0 ms, CLS 0.

## Declared claim commands

All 21 exact commands in `.factory/claims.json` were run independently after
`npm ci`; all passed. Their command output is retained in
`/work/.evidence/webhook-quiet-hours-verify-6/claim-*.log`.

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

The complete clean-checkout gate also passed:

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
The build produced `dist/` with 35.62 KB JavaScript (11.76 KB gzip) and
18.87 KB CSS (5.07 KB gzip).

## Earlier findings

Every earlier finding is closed and rechecked: live identity and immutable
asset caching (verification 1); no-config startup (verification 2); paid-panel
contrast, keyboard tabs, announcements, and persistent touch targets
(verification 3); claims inventory, first-screen clarity, isolated demo,
pre-auth limiting, discovery files, and designed 404 (verification 4); and
durable `/data` state, checkout, one-replica live allowance, complete claims,
demo recovery, 200% reflow, route focus/announcement, and legal touch targets
(verification 5). There are no remaining findings of any severity.

## Evidence

- Browser transcript: `/work/.evidence/webhook-quiet-hours-verify-6/live-browser.json`
- Phone and desktop landing/demo screenshots:
  `/work/.evidence/webhook-quiet-hours-verify-6/`
- Lighthouse JSON:
  `/work/.evidence/webhook-quiet-hours-verify-6/lighthouse.json`
- Full test output:
  `/work/.evidence/webhook-quiet-hours-verify-6/npm-test.log`

**Final verdict: PASS.**
