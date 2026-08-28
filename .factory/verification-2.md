# Verification 2 — FAIL

**Verified candidate:** `a607aa5bd48d24a4b741db08e0438db76ece6469`  
**Public URL:** `https://webhook-quiet-hours.sociobot.in`  
**Date:** 2026-08-28 UTC  
**Result:** **FAIL — do not accept this candidate under the supplied backend-service contract.**

## Release identity

The prior deployment-identity failure is resolved. A fresh public request at
2026-08-28 03:05 UTC returned HTTP 200 from
`https://webhook-quiet-hours.sociobot.in/health`:

```json
{"build_sha":"a607aa5bd48d24a4b741db08e0438db76ece6469","status":"ok"}
```

The public deployment therefore matches the requested candidate exactly.

## Blocking defect

### BLOCKER — production container cannot start with only `PORT`

The mandatory runtime contract says the container must start and serve with no
configuration other than `PORT`; secret-like values must be generated with a
CSPRNG and persisted on first boot. This candidate instead makes `ADMIN_TOKEN`
and `DATA_ENCRYPTION_KEY` mandatory whenever `APP_ENV=production`.

Fresh reproduction against the candidate's release binary:

```sh
env -i APP_ENV=production PORT=18081 target/release/webhook-quiet-hours
# Error: Config("ADMIN_TOKEN is required in production")
```

Source evidence is `src/lib.rs:48-69`; it then similarly rejects a missing
`DATA_ENCRYPTION_KEY`. `Dockerfile` sets `APP_ENV=production`, so its normal
runtime path has this failure. The application neither generates/persists the
admin or encryption secret nor logs which configuration was generated versus
supplied. The deployed instance is evidently being supplied additional secrets,
but that does not satisfy the factory's required zero-configuration deployment
contract. This is release-blocking.

## Fresh checks that passed

### Clean checkout and build gates

- Clean checkout was at exactly `a607aa5…`; `npm ci` completed with 0 reported
  vulnerabilities.
- `npm test`: passed — 3 Vitest tests and 6 Rust tests.
- `npm run check`: passed — TypeScript, rustfmt, and Clippy with warnings
  denied.
- `npm run build`: passed; generated `dist/`.
- `cargo build --release --locked`: passed.
- The worker has no Docker, Podman, Buildah, or Nerdctl, so the Dockerfile
  assembly itself could not be executed. Its exact Node/Vite and locked Rust
  production build stages were executed above; container-runtime behavior was
  exercised directly with the resulting release binary.

Bundle measurements: initial JS 26,233 B raw / 9,066 B gzip; CSS 15,691 B raw
/ 4,506 B gzip; mobile hero WebP 23,638 B. These are within the stated
200 KB/50 KB/300 KB budgets.

### Production-mode backend end-to-end

Using a disposable SQLite database, generated 32-byte encryption key,
`ADMIN_TOKEN`, candidate `BUILD_SHA`, and the built `dist/`:

- `/health` returned the exact candidate SHA; unauthenticated summary returned
  401.
- A one-character alias returned 400 with actionable validation; creating a
  signed alias returned its one-time HMAC secret and private URL.
- Unsigned ingress returned 401. Two correctly HMAC-SHA256-signed
  `invoice.failed`/500 events returned 202 and compressed into one fingerprint
  with total count 2; detail decrypted the retained `evt-qa-marker` payload.
- Invalid classification returned 400; promotion to high and acknowledgement
  both returned 204. Invalid `25:00` quiet hours returned 400; valid overnight
  quiet/deletion/escalation settings returned 204.
- Exactly 262,144 bytes returned 202; 262,145 bytes returned 400. One hundred
  concurrent correctly signed ingress requests yielded 100/100 HTTP 202.
- CSV export returned 200 with `text/csv; charset=utf-8`. Searching the raw
  SQLite file did not find `evt-qa-marker`, while the authenticated detail API
  decrypted it, confirming retained raw payload ciphertext at rest.

No external notification receiver was configured, avoiding an external
side-effect; this leaves real Slack-compatible delivery unexercised.

### Public browser, privacy, PWA, headers, and performance

- Public root at 1440×900 and 390×844: HTTP 200, one `h1`, `main`, `lang=en`,
  no horizontal overflow, no page/console errors, no third-party first-load
  requests, and an operable visible 3 px focus ring on the keyboard skip link.
  Reduced-motion mode yielded `transform: none` for the specimen.
- Fresh Playwright axe audits at both viewports found zero serious or critical
  findings. Public `/privacy` and `/terms` each returned 200 with one `h1`,
  one `main`, zero page/console errors, and zero serious/critical axe findings.
- Local 390 px authenticated UI: invalid-token recovery appeared as expected;
  valid login opened the dashboard; keyboard/open dialog focused the alias-name
  field; creation reached the one-time-secret screen; no serious/critical axe
  findings or horizontal overflow. The deliberate invalid-login XHR produced
  expected browser 401 console resource messages; normal public loads did not.
- Service worker controlled the page after reload; a 390 px offline reload
  still rendered the landing `h1` and `main` with no errors.
- Live security policy includes CSP restricted to self plus the documented
  Sociobot billing origins, `nosniff`, `X-Frame-Options: DENY`, and
  `Referrer-Policy: no-referrer`. Hashed JS/CSS use
  `public, max-age=31536000, immutable`; non-fingerprinted hero image uses one
  day; shell, legal routes, and `/sw.js` use `no-cache`.
- Lighthouse mobile/public root: Performance 99, Accessibility 100, Best
  Practices 100, SEO 92; LCP 1,712 ms, TBT 0 ms, CLS 0.

## Required remediation

Change production startup so a CSPRNG admin token and AES-256 key are generated
and persisted under the data volume when absent, with supplied environment
values only overriding them. Start successfully with only `PORT`; add a
non-secret startup log identifying generated versus supplied values; cover this
in an integration/container test. Re-run an exact container build and this
verification after the correction.

## Reproduce

```sh
npm ci
npm test
npm run check
npm run build
cargo build --release --locked
curl -sS https://webhook-quiet-hours.sociobot.in/health
env -i APP_ENV=production PORT=18081 target/release/webhook-quiet-hours
```
