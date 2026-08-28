# Verification 1 — FAIL

**Verified commit:** `d854693319e5f9cf993dff39a51f56ca82d4a8e3`  
**Public URL:** `https://webhook-quiet-hours.sociobot.in`  
**Date:** 2026-08-28 UTC  
**Result:** **FAIL — do not accept or promote this candidate.**

## Blocking defect

### BLOCKER — public deployment identity is not the candidate

Fresh `GET https://webhook-quiet-hours.sociobot.in/health` returned HTTP 200
but its body was:

```json
{"build_sha":"742c55ba4df05cb6fac46a5a6761c54448b6502f","status":"ok"}
```

It does **not** report requested candidate
`d854693319e5f9cf993dff39a51f56ca82d4a8e3`. `d854693` changes only
`.factory/handoff.md`, and its product source is byte-for-byte unchanged from
`742c55b` (excluding that handoff), but that does not establish that the live
deployment is the requested candidate. The health/build-identity acceptance
criterion therefore fails. Deploy an image built/tagged with `d854693` and
verify `/health` again before release.

## Other defects

### Medium — hashed static assets are not cacheable immutably

Public root, JS, CSS, image, and service-worker responses have no
`Cache-Control` header. The hashed JS/CSS assets (`index-D7dkerDD.js`,
`index-CTPxq1pn.css`) should be served with a long-lived immutable policy.
This misses the stated caching requirement and causes avoidable repeat
transfers. Security headers below were otherwise present.

## What passed

### Clean local gates

- `npm ci`: passed; 0 vulnerabilities.
- `npm test`: passed — 3 Vitest assertions and 5 Rust tests.
- `npm run check`: passed — strict TypeScript, rustfmt, Clippy with
  `-D warnings`.
- `npm run build`: passed; Vite output in `dist/`.
- `cargo build --release --locked`: passed.
- Docker/Podman were not installed in this verification worker, so the
  Dockerfile itself could not be executed. Its two build stages were covered
  by the exact frontend and release-Rust commands above.

Bundle evidence: JS 26,233 B raw / 9,066 B gzip; CSS 15,691 B raw / 4,506 B
gzip; mobile hero WebP 23,638 B. All are within the specified budgets.

### Production-configured local backend

Ran the release binary with `APP_ENV=production`, a generated 32-byte base64
encryption key, disposable SQLite database, `BUILD_SHA=d854693…`, and the
fresh `dist/`. `/health` returned that exact SHA.

- Unauthenticated `/api/summary`: 401; authenticated empty summary: 200.
- Invalid one-character alias: 400 with actionable validation text.
- Created a signed alias: 201, one-time HMAC secret and private URL returned.
- Unsigned ingress to it: 401; correctly HMAC-signed ingress: 202.
- Repeating the representative `invoice.failed`/500 payload gave one stable
  fingerprint with count 2; decrypted detail returned the retained payload.
- Invalid classification: 400; high promotion and acknowledgement: 204.
- Invalid quiet-time/settings payload: 400; valid quiet, retention, and
  escalation settings: 204.
- 262,145-byte payload: 400; exact 262,144-byte payload: 202.
- 100 simultaneous valid ingress requests: 100 HTTP 202, 0 failures.
- CSV export: 200 `text/csv; charset=utf-8`.
- The tested raw payload marker (`evt-1`) was absent from SQLite while the API
  could decrypt it, confirming encrypted retained payload storage. Metadata
  such as event type/fingerprint is intentionally queryable.

No actual notification destination was supplied, so no external Slack-style
delivery was generated.

### Browser, accessibility, privacy, and PWA checks

- Live desktop (1440×900) and mobile (390×844): one h1 and main landmark,
  no horizontal overflow, no page or console errors, and only same-origin
  runtime requests on the unauthenticated landing page.
- Local authenticated dashboard at both viewports: login, fingerprint detail,
  high-severity acknowledgement, invalid-token recovery, and keyboard focus
  all worked. Focused controls reported a visible solid outline.
- Axe Playwright audit: zero serious/critical findings on live landing at
  desktop/mobile and authenticated local dashboard at desktop/mobile.
- Reduced-motion context applied `transform: none` to the desktop specimen.
- The live service worker installed, controlled after reload, cached `/` and
  both hero WebPs, and an offline reload rendered successfully in Chromium.
- Live privacy/terms routes returned 200. No CDN fonts, analytics, or
  third-party landing-page requests were observed. The app's CSP restricts
  scripts/styles/images to self and permits only the documented Sociobot
  billing origins for connections/forms; `nosniff`, `DENY` framing, and
  `no-referrer` were also returned.
- Lighthouse mobile against the public root: Performance 99, Accessibility
  100, Best Practices 100, SEO 92; LCP 1,147 ms, TBT 138 ms, CLS 0.

## Reproduce

```sh
npm ci
npm test
npm run check
npm run build
cargo build --release --locked
curl -sS https://webhook-quiet-hours.sociobot.in/health
```

Release acceptance requires the final command to identify
`d854693319e5f9cf993dff39a51f56ca82d4a8e3`, not `742c55b…`.
