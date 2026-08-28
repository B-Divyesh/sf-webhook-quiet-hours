# Independent verification 3 — FAIL

**Candidate:** `c2baeb81b8e9b70385e8669a6e6143694debbb7b`  
**Public URL:** `https://webhook-quiet-hours.sociobot.in`  
**Verified:** 2026-08-28 UTC  
**Result:** **FAIL — do not accept or promote this candidate.**

The previous deployment-only and zero-configuration startup failures are fixed.
Fresh public evidence identifies the deployment as this exact candidate, and the
core webhook workflow passes. Acceptance is blocked by a serious axe finding on
the authenticated Aliases screen, which is part of the purchase-restore flow.

## Release identity and deployment match

At 2026-08-28 05:09 UTC, the public health endpoint returned HTTP 200:

```json
{"build_sha":"c2baeb81b8e9b70385e8669a6e6143694debbb7b","status":"ok"}
```

The live `index.html`, hashed JavaScript, hashed CSS, both hero images, and
`sw.js` were downloaded and compared byte-for-byte with the fresh candidate
build; every SHA-256 matched. The factory URL verifier returned HTTP 200 in
717 ms with the expected title, `lang=en`, one `h1`, a `main`, complete image
alt text, and no console/page errors.

## Defects

### HIGH — authenticated purchase-restore panel has serious contrast failures

A fresh Playwright 1.58.2 + axe audit of the authenticated **Aliases** panel
reported `color-contrast` with impact `serious` at both 390×844 and 1440×900:

- `Restore license`: foreground `#F4F0E5` on `#FBF8EF`, **1.07:1**; required
  4.5:1. The control is visually almost blank and is the only way to restore a
  purchase on another device.
- `One-time unlock · $39`: foreground `#71865F` on `#172019`, **4.19:1**;
  required 4.5:1.

The cause is the light `.secondary` button treatment inheriting the paid card's
paper-colored text while retaining a paper-colored background. Public and legal
pages, dashboard, populated ledger, and modal audits had zero axe violations,
but those checks do not cover this authenticated state. The supplied contract
requires all serious/critical axe findings to be fixed, so this is
release-blocking.

### MEDIUM — tablist does not support arrow-key navigation

The three dashboard controls declare `role="tab"` inside `role="tablist"`, but
ArrowRight from the focused/selected Observations tab leaves both focus and
selection on Observations. Tab and Enter remain usable, but this misses the
explicit keyboard requirement for arrow-operated custom widgets.

### MEDIUM — action status announcements are removed before they can be perceived

Acknowledgement, settings save, alias deletion, classification save, and manual
digest call `showStatus(...)` and then immediately call `loadDashboard()`. That
synchronously replaces the live-region node with a new empty node. At 300 ms,
the acknowledgement, settings, and “Nothing pending” digest messages were all
absent. State changes provide some indirect feedback, but the no-op manual
digest has no perceivable result.

### MEDIUM — three persistent mobile links miss the 44 px touch-target baseline

At 390 px, measured visible hit boxes were 23 px high for the home/brand link
and 21 px high for Privacy and Terms. Their widths were 165, 49, and 40 px.
This misses the supplied 44×44 CSS px touch-target requirement.

## Clean checkout and build gates

Verification used a detached clean worktree at the exact candidate.

- `npm ci`: passed; 55 packages, 0 vulnerabilities.
- `npm audit --audit-level=high`: passed; 0 vulnerabilities.
- `npm test`: passed — 3 Vitest tests, 6 Rust unit/router tests, and 1
  process-level startup/restart integration test.
- `npm run check`: passed — TypeScript, rustfmt, and Clippy with warnings denied.
- `npm run build`: passed and produced `dist/`.
- `cargo build --release --locked`: passed.
- Bundle sizes: JS 26,265 B raw / 9,077 B gzip; CSS 15,691 B raw / 4,506 B
  gzip; mobile hero 23,638 B. There are no font files. All are below budget.

Docker, Podman, Buildah, and Nerdctl are unavailable in this worker, so the
Dockerfile could not be assembled locally. Its exact locked Rust release and
Vite stages were run, and the live deployment's exact SHA plus byte-identical
web assets provide deployed-artifact evidence.

## Backend and end-to-end evidence

A disposable release-binary instance was exercised with a real SQLite file and
a local Slack-compatible POST receiver.

- A first boot with only `PORT` generated independent admin/encryption secrets,
  persisted both with mode 0600, logged only `generated` provenance, and served
  `/health`. Restart in `APP_ENV=production` reused byte-identical files, logged
  `persisted`, and accepted the original admin token. Neither secret appeared in
  logs.
- Missing admin auth returned 401 with actionable JSON; authenticated empty
  summary and unknown-API 404 behaved correctly.
- Alias names of 1 and 61 characters returned 400. A signed alias returned a
  one-time receiver URL and HMAC secret. Wrong key, missing signature, and bad
  signature returned 401; two valid HMAC-SHA256 events returned 202 and one
  stable fingerprint with count 2.
- One hundred concurrent signed events returned 100/100 HTTP 202. Summary then
  reported 102 events, 1 fingerprint, and 101 compressed repeats. One hundred
  concurrent health requests also passed.
- Exactly 262,144 bytes returned 202; 262,145 returned 400 with the documented
  256 KB message.
- Invalid quiet time, UTC offset, digest interval, retention, URL scheme,
  severity, and acknowledgement-target boundaries returned 400. Valid
  overnight quiet hours and boundary values persisted.
- Promoting the fingerprint to high severity sent a real JSON notification to
  the local receiver with the configured single runbook link; acknowledgement
  returned 204 and cleared the pending count.
- Authenticated detail decrypted `evt-qa-marker`; searches of raw SQLite found
  neither that payload marker, the HMAC secret, nor the notification URL.
- CSV export returned 200 with `text/csv; charset=utf-8`. A process restart kept
  endpoints, fingerprints, settings, and 103 retained events.

## Browser, privacy, PWA, headers, and performance

- Live 1440×900 and 390×844 landing pages had one `h1` and `main`, no overflow,
  missing alt, third-party first-load request, console error, or page error.
  The first Tab exposed a 48.8 px skip link with a 3 px focus ring.
- Dark mode and reduced-motion mode passed; the specimen transform became
  `none`. Privacy and Terms rendered at direct URLs with clean axe and console
  results.
- Authenticated mobile flow recovered from an invalid token, rendered the empty
  state, opened the alias dialog by keyboard with focus in the name field,
  showed native invalid-input feedback, created an alias, copied its one-time
  secret, accepted a signed event, decrypted its detail, promoted and
  acknowledged it, saved quiet-hour settings, and confirmed/cancelled deletion.
- The service worker controlled the live page, `registration.update()`
  completed, and a 390 px offline reload rendered the shell.
- A returned license token was stored under the required key and stripped from
  the URL; one verification unlocked the product, a reload used the daily
  cache, a mocked revoked verdict locked it again, and paste-to-restore showed
  an invalid-license error. The real billing verify endpoint returned the
  documented invalid verdict and correct CORS origin for a harmless QA token.
- With no license token, every first-load request was same-origin. No analytics,
  CDN font, third-party script, or cross-tenant telemetry request was observed.
- Root, legal routes, SPA fallback, and `sw.js` use `no-cache`; hashed JS/CSS use
  `public, max-age=31536000, immutable`; the hero uses one-day caching. CSP is
  self-only except the documented billing API, with `nosniff`, DENY framing,
  and `no-referrer`.
- Fresh mobile Lighthouse: Performance 100, Accessibility 100, Best Practices
  100, SEO 92; LCP 933 ms, TBT 52.5 ms, CLS 0, total transfer 38,639 B. The
  public Lighthouse score does not cover the authenticated contrast failure.

## Required remediation

Give the paid card's Restore control and eyebrow text AA-compliant colors in
both themes, then run axe on the authenticated Aliases panel at desktop and
mobile. Add ArrowLeft/ArrowRight/Home/End behavior and roving focus to the tab
widget, preserve one stable live region across dashboard rerenders, and enlarge
the persistent mobile link hit areas to at least 44×44 px. Re-run this full
verification against a newly identified deployment.

## Reproduce

```sh
npm ci
npm audit --audit-level=high
npm test
npm run check
npm run build
cargo build --release --locked
curl -sS https://webhook-quiet-hours.sociobot.in/health
```

Authenticated axe reproduction requires starting the release binary with a
disposable database, opening the dashboard with its admin token, selecting
Aliases, and running axe at 390×844 and 1440×900.
