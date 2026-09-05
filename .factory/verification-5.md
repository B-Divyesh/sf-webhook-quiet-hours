# Verification 5: Group webhook failures before Slack — FAIL

- Verdict: **FAIL — do not accept or promote this candidate.**
- Implementation candidate: `994aa998e94b981815cc790d538d79c4358d98fa`
- Live URL: `https://webhook-quiet-hours.sociobot.in`
- Verified: 2026-09-05 UTC
- Findings: **8**
- Untested public claims: **10**

The live health response identifies the exact implementation candidate. A fresh
local build also produced HTML, JavaScript, and CSS that match the live files
byte for byte. The supplied live revision is
`sf-webhook-quiet-hours--0000012`, with image digest
`sha256:ac8e4c065fec17d7dda53d430f236e6ef596250ce60fa59b240d02a0da7d631d`.

## Findings

### Blocker — production state does not use the fleet `/data` mount

The Dockerfile sets
`DATABASE_URL=sqlite:///app/data/quiet-hours.db?mode=rwc` and declares
`VOLUME ["/app/data"]`. The work order provides durable storage at `/data`.
With only the factory-provided `PORT`, the deployed process therefore writes
SQLite, the admin token, and the encryption key outside the durable mount.
The restart test reuses one temporary working directory, so it does not prove
survival across a fleet redeploy. This risks losing receiver state and the key
needed to decrypt retained payloads.

### High — the public purchase link returns 404

Both the landing page and demo advertise the $39 Field Station purchase at
`https://api.sociobot.in/api/v1/products/webhook-quiet-hours/checkout`.
A fresh GET with redirects disabled returned HTTP 404 and
`{"error":"enabled factory product","status":404}`. The declared
`one-time-price` test checks only the link text and `href`; it never opens the
link. The paid path is not usable.

### High — the live 40-request dashboard burst is not enforced

The in-process claim test passes, but the public service does not enforce the
documented allowance across live replicas. From one fixed
`X-Forwarded-For` address:

- 60 concurrent unauthenticated `/api/summary` requests returned 60×401 and
  no 429.
- A separate 120-request burst returned 118×401 and 2×429.
- Both observed 429 responses included `Retry-After: 19`.
- A different client address remained independent with 401, and 100 concurrent
  health checks returned 100×200.

The documented burst is 40 per source IP. An instance-local limiter behind
multiple live replicas permits substantially more than that public allowance.

### High — the claims inventory omits public promises

`.factory/claims.json` contains ten commands and all ten pass, but public copy
in the live site, legal pages, and README makes ten additional promises without
one exact claim test each:

1. Demo workspaces expire after 24 hours.
2. Signing secrets and notification URLs are encrypted at rest.
3. Retention cleanup deletes payloads on schedule.
4. Slack-compatible digests send, high severity sends immediately, and
   record-only items do not notify.
5. The dashboard is responsive and keyboard accessible.
6. Free and paid alias/retention limits are enforced.
7. A purchase includes every future self-hosted release.
8. Ingress accepts 256 KB and rejects larger bodies.
9. Ingress enforces 100 requests/second with a 200-request burst per source IP.
10. The server shuts down gracefully.

Some behavior appears in broad regression tests or source, but the claims
contract requires each public promise to be listed and directly asserted.
“Every future self-hosted release” is not testable in the sandbox and should
not be promised.

### Medium — initial demo network failure remains stuck on loading

In a fresh phone context, aborting `POST /api/demo/session` left the page on
“Reading observations…” and emitted an uncaught `Failed to fetch` page error.
The promised “Sample workspace could not open” state and retry action did not
render. Restoring the network and manually reloading recovered the demo.

### Medium — the populated demo does not reflow at 200% text size

At a 390 px viewport with root text enlarged to 200%, the document measured
548 px wide. Summary cells, tabs, and ledger count/severity columns extended
off screen. The normal-size phone layout has no overflow, but the required
200% text-resize path forces horizontal scrolling.

### Medium — route changes do not move focus or announce the new page

Opening Privacy by keyboard sets the correct URL and title, but focus lands on
`body`; the `h1` has no focus target and the live region is empty. Back
navigation also returns focus to `body`. Navigation uses full reloads rather
than the required history/focus behavior.

### Medium — four inline links miss the 44 px touch target

At 390 px, the landing-page “privacy notice” and “terms” links are 20 px high.
The “Return to Webhook Quiet Hours” link is also 20 px high on both legal
pages. The previously reported brand, footer, demo, and paid-action targets are
now at least 44 px.

## First screen and demo

Before scrolling in fresh 390×844 and 1440×900 contexts:

- Job: “Group webhook failures before they reach Slack.”
- Audience: small engineering teams that need urgent failures without every
  repeat going to Slack.
- First action: “Try it with sample data.”

All three were visible in both viewports. One keyboard action opened `/demo`.
The demo showed one “Deploy monitor” alias, 18 deliveries, three fingerprints,
and 15 compressed repeats. The “Demo — sample data, nothing is saved” label
persisted through detail inspection, acknowledgement, and reload. Reset
restored the high item. Demo storage used only
`demo:webhook-quiet-hours:*`; no real admin-token namespace appeared. The
declared local isolation test also proved the real SQLite summary remained
empty. “Start for real” discarded the workspace and returned to the landing
page.

## Declared claim commands

Every exact command from `.factory/claims.json` passed independently:

| Claim | Command | Result |
| --- | --- | --- |
| demo-sandbox | `npx playwright test --grep @claim:demo-sandbox` | Pass |
| repeat-compression | `npx playwright test --grep @claim:repeat-compression` | Pass |
| csv-export | `npx playwright test --grep @claim:csv-export` | Pass |
| privacy-same-origin | `npx playwright test --grep @claim:privacy-same-origin` | Pass |
| one-time-price | `npx playwright test --grep @claim:one-time-price` | Pass, but incomplete; checkout is 404 |
| encrypted-payloads | `cargo test claim_payloads_are_encrypted_at_rest` | Pass |
| signed-ingress | `cargo test signed_webhook_is_received_and_grouped` | Pass |
| quiet-window | `cargo test overnight_quiet_window_works` | Pass |
| api-rate-limit | `cargo test unauthenticated_api_requests_are_rate_limited_before_authentication` | Pass locally; false at the advertised live allowance |
| zero-config-start | `cargo test --test runtime_startup production_binary_boots_with_only_port_and_reuses_persisted_secrets` | Pass |

## Other verification that passed

- `npm ci`: 59 packages; `npm audit --audit-level=high`: zero
  vulnerabilities.
- `npm test`: 3 Vitest, 10 Rust unit/router, 1 runtime integration, and 17
  Playwright tests passed.
- `npm run check`, `npm run build`, and
  `cargo build --release --locked` passed.
- Bundle: 34.44 KB JavaScript (11.42 KB gzip), 17.88 KB CSS (4.88 KB gzip),
  and 23.64 KB mobile hero.
- Factory URL verification passed in 568 ms with no load console/page errors,
  one `h1`, `lang=en`, `main`, complete image alt text, and labelled
  buttons.
- Fresh phone and desktop browser checks passed the normal demo flow, light and
  dark axe serious/critical audits, keyboard tab behavior, dialog focus return,
  normal-size reflow, reduced motion, internal links, route titles, legal
  pages, same-origin request capture, and service-worker update/offline reload.
- Mobile Lighthouse: 100 Performance, 100 Accessibility, 100 Best Practices,
  100 SEO; LCP 1,080 ms, TBT 0 ms, CLS 0.
- `robots.txt` and `sitemap.xml` return the correct types. Shell and service
  worker use `no-cache`; hashed JS/CSS use one-year immutable caching.
  Security headers are present.
- The deliberate unknown path correctly returned HTTP 404 with the designed
  recovery page. This expected 404 is not a defect.
- A disposable release binary passed healthy/unauthorized/authorized requests,
  short-alias validation, unsigned and bad-signature rejection, two signed
  accepted events grouped together, 262,144-byte acceptance, 262,145-byte
  rejection, invalid settings, valid recovery, encrypted payload inspection,
  owner-only generated secret files, and absence of credentials in logs.

The local `dist/index.html`, JavaScript, and CSS SHA-256 values exactly match
the live responses. No real tenant data, notification destination, license, or
payment was created or changed.

## Earlier finding disposition

| Earlier finding | Current disposition |
| --- | --- |
| Verification 1: live identity mismatch | Resolved: live health returns `994aa998…`; built and live web assets match. |
| Verification 1: immutable caching absent | Resolved for hashed JS/CSS. |
| Verification 2: startup requires secrets | Resolved in the process test; secrets are generated and reused. Durable fleet placement is still wrong under the new blocker above. |
| Verification 3: paid-panel contrast | Resolved in phone/desktop, light/dark axe checks. |
| Verification 3: tab arrow keys | Resolved. |
| Verification 3: status announcements replaced | Resolved by the regression suite. |
| Verification 3: persistent link targets below 44 px | Resolved for the reported header/footer links; other inline links remain undersized. |
| Verification 4: no claims file | Partly resolved: ten claims exist and pass, but the inventory remains incomplete. |
| Verification 4: no demo and unclear first screen | Resolved for the normal path. Network-failure recovery remains broken. |
| Verification 4: pre-auth traffic bypasses the limiter | Resolved in one process; the public multi-replica allowance still exceeds the claim. |
| Verification 4: paid link target, discovery metadata, 404 | Target size and site files are resolved; the checkout destination itself is 404. |
| Verification 4: fixed service-worker cache | Resolved with a versioned cache and hashed asset precache. |

## Required action

Keep the release blocked. Store all persistent state under `/data`, make the
checkout route usable, enforce the published allowance across replicas, cover
or remove every public claim, and repair the three remaining interface
failures. Then deploy a new implementation and repeat independent verification.
