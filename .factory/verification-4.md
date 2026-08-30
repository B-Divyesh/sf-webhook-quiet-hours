# Independent verification 4 — FAIL

- Candidate: `123678eb3a4b6341fc84d0c0eb018f1cb12e6cab`
- Live URL: `https://webhook-quiet-hours.sociobot.in`
- Verified: 2026-08-30 UTC
- Verdict: **FAIL — release blocked.**

## Mandatory claims and first-read gate

`/.factory/claims.json` does not exist in this clean candidate. Consequently there were no declared claim commands to run from the demo entry point. This is an explicit release blocker.

Cold live-page read rendered: “Let webhook failures break the quiet. Not every webhook.” It offers only a Server admin token field and Open field log. It does not name small engineering teams in plain words, and it has no visible **Try it with sample data** action. `/demo` returns the SPA landing HTML, not an isolated seeded workspace. There is no demo banner, reset/start-for-real control, `.factory/demo.md`, or demo storage/tenant implementation. This independently fails the first-read and one-click-demo gates.

## Local quality gates

- `npm ci` — pass; 60 packages, 0 vulnerabilities reported.
- `npm test` — pass: Vitest 3/3, Rust 8 tests (including runtime startup), Playwright 5/5.
- `npm run check` — pass: TypeScript, rustfmt, Clippy with warnings denied.
- `npm run build` — pass; `dist/` produced. Initial JS is 26,593 B (9.22 kB gzip), CSS 16,081 B (4.56 kB gzip), and the 820 px hero WebP is 62,432 B.
- `cargo build --release --locked` — pass.
- Docker was unavailable in the verifier container (`docker: command not found`), so an image build/run could not be independently exercised.

## Functional receiver checks

Started the candidate with a fresh SQLite database and built `dist/`. Empty authenticated summary returned all zeroes. Invalid alias input returned 400. A signed alias was created; invalid HMAC was rejected; two correctly HMAC-SHA256-signed `deployment.failed` events became one fingerprint (`total_count: 2`). An invalid rule and invalid quiet hours returned 400; a high-severity update, third event, acknowledgement, valid settings recovery, and CSV export all succeeded. The existing runtime test also passed restart/persisted-secret behaviour.

## Live identity, privacy, headers and cache

`GET /health` returned `{"build_sha":"123678eb3a4b6341fc84d0c0eb018f1cb12e6cab","status":"ok"}`. Fresh candidate and live HTML, JS, and CSS SHA-256 values exactly match, so this is not a stale deployment.

Cold live-page requests were only same-origin (`/`, hashed JS/CSS, local hero WebP); authenticated local browser requests were only same-origin `/api/*`. No page or console errors occurred. The paid-license path was not invoked because no license was available. CSP permits self plus the documented Sociobot billing origins. Responses send CSP `frame-ancestors 'none'`, `nosniff`, `X-Frame-Options: DENY`, and `Referrer-Policy: no-referrer`. Hashed JS/CSS are one-year immutable; image cache is one day; HTML/SW is no-cache. `/robots.txt` and `/sitemap.xml` both return the HTML shell (`text/html`), not valid discovery files.

## Accessibility, responsive and interaction checks

The live cold page had no console/page errors. Local real-server dashboard tests at desktop and 390×844 mobile found no axe serious or critical violations. Existing Playwright tests pass roving tabs and status/focus regressions. Reduced-motion CSS and a skip link are present. One target failure was measured on both widths: visible inline `Buy for $39 once` is 137×19 CSS px, below the 44 px minimum.

## Rate-limit evidence

With one authenticated local client and a fixed `X-Forwarded-For`, 60 parallel summary requests yielded **40 × 200** then **20 × 429**, each sampled 429 carrying `Retry-After: 19`; observed allowance is 20 requests/s, burst 40. The live admin token was unavailable for that authenticated check.

Separately, 60 parallel unauthenticated live `/api/summary` requests from one fixed forwarded IP all returned 401 and none returned 429. Authentication runs before the API governor, so rejected public API traffic is not rate-limited. This contradicts the requirement that every server-side endpoint be rate limited.

## Defects

### Blocker

1. Missing `.factory/claims.json`; no claim-test execution is possible.
2. No isolated one-click sample-data demo: no first-screen action, seeded `/demo`, banner/reset/real-data separation, or demo documentation.
3. Cold-page clarity fails: metaphorical headline, no plainly named intended user, and no way for a new visitor to try the product.

### High

4. Unauthenticated API requests bypass rate limiting: 60 same-client live calls all got 401 rather than eventually 429 plus `Retry-After`.

### Medium

5. The inline paid link is only 19 px tall on desktop and 390 px mobile.
6. Site discovery/metadata is incomplete: robots and sitemap are SPA HTML; canonical, Open Graph/Twitter metadata, Apple touch icon, and a real 404 route are absent.

### Low

7. Service worker uses a fixed `quiet-hours-shell-v1` cache and does not precache hashed JS/CSS, so reliable update/offline-shell behaviour is not established. There is no advertised offline claim.

## Required disposition

Do not release. Add and pass required claim tests from a real demo entry point, implement the isolated sample demo and plain first screen, move rate limiting outside authentication, and address the medium defects before re-verification.
