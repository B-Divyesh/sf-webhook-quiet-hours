# Webhook Quiet Hours

Webhook Quiet Hours is a small, self-hosted receiver for engineering teams that
need actionable webhook failures without sending every event to Slack. It gives
each source a private endpoint alias, verifies optional HMAC signatures, groups
repeat event shapes into fingerprints, and sends quiet-hour-aware digests. A
configured high-severity fingerprint bypasses the digest and carries one review
link.

It is an aggregation and alerting aid—not a webhook delivery/retry proxy or a
general automation platform.

## What v1 includes

- Private endpoint keys and optional HMAC-SHA256 verification compatible with
  `X-Hub-Signature-256` or `X-Webhook-Signature`.
- AES-256-GCM encryption for payloads, signing secrets, and notification URLs.
- Stable fingerprint aggregation using endpoint, event type, payload shape, and
  status/error code.
- Normal, high, and record-only rules with acknowledgement targets.
- Configurable quiet window, UTC offset, digest cadence, escalation link, and
  automatic payload deletion.
- Slack-compatible notification webhook, manual digest, CSV export, and
  responsive/keyboard-accessible dashboard.
- Free use with one alias and seven-day retention. The optional $39 one-time
  Field Station browser unlock adds unlimited aliases and up to 90-day retention
  through the Sociobot billing API. Signing, escalation, and export are not gated.

There is no analytics, cross-tenant telemetry, third-party script, or CDN font.

## Run locally

Requirements: Node 22+, Rust 1.89+, and a C toolchain.

```sh
npm ci
npm run build
cargo run
```

Open `http://localhost:8080`. On first boot the server generates an admin token
and encryption key with the operating system CSPRNG and writes them to
`data/admin-token` and `data/encryption-key` with owner-only permissions. Paste
the value from `data/admin-token` into the dashboard login. Keep the whole
`data/` directory private and backed up. For live frontend reload, run
`npm run dev` alongside `cargo run` and open port 5173.

Quality commands:

```sh
npm test          # Vitest plus Rust tests
npm run check     # TypeScript, rustfmt, Clippy
npm run build     # reproducible frontend output in dist/
cargo build --release --locked
```

## Production configuration

The container starts with no runtime configuration other than `PORT`. At first
boot it generates and persists both secrets beside the SQLite database under
`/app/data`; subsequent boots reuse them. Explicit environment values override
the persisted defaults. Startup emits only whether each value was `generated`,
`persisted`, or `supplied`—never the secret itself.

| Variable | Required | Default | Purpose |
| --- | --- | --- | --- |
| `ADMIN_TOKEN` | no | CSPRNG value persisted beside SQLite | Dashboard bearer token |
| `DATA_ENCRYPTION_KEY` | no | CSPRNG value persisted beside SQLite | Base64 encoding of exactly 32 random bytes |
| `DATABASE_URL` | no | `sqlite://data/quiet-hours.db?mode=rwc` | SQLite connection URL |
| `PUBLIC_URL` | no | `http://localhost:8080` | Base used in generated hooks/review links |
| `PORT` | no | `8080` | HTTP listener |
| `BUILD_SHA` | no | `development` | Returned by `/health` |
| `DIST_DIR` | no | `dist` | Built frontend directory |
| `VITE_BILLING_BASE` | build only | `https://api.sociobot.in` | Use `https://pilot-api.sociobot.in` for staging |

To override the generated encryption key, create one without storing it in the
repository:

```sh
openssl rand -base64 32
```

Build and run the production image. Pass the exact source revision at build
time so `/health` identifies the image that is actually running:

```sh
docker build \
  --build-arg BUILD_SHA="$(git rev-parse HEAD)" \
  -t webhook-quiet-hours .
docker run --rm -p 8080:8080 \
  -v webhook-quiet-hours-data:/app/data \
  webhook-quiet-hours
```

Read the generated login token with
`docker exec <container> cat /app/data/admin-token`. In a managed deployment,
read the same file from the mounted data volume or supply `ADMIN_TOKEN` through
its secret manager. Supply `PUBLIC_URL` when generated receiver URLs must use a
public hostname; it is not required for startup.

Terminate with SIGTERM for graceful shutdown. Back up the SQLite volume and the
two generated secret files together; retained ciphertext cannot be recovered
without `encryption-key`, and dashboard access depends on `admin-token`.

## Sending a signed webhook

After creating an alias, copy its receiver URL and one-time HMAC secret. Compute
HMAC-SHA256 over the exact raw body and send the hexadecimal digest:

```sh
body='{"type":"invoice.failed","status":500,"id":"evt_123"}'
signature="$(printf %s "$body" | openssl dgst -sha256 -hmac "$SIGNING_SECRET" -hex | sed 's/^.* //')"
curl --fail-with-body -X POST "$RECEIVER_URL" \
  -H "X-Webhook-Signature: sha256=$signature" \
  --data-binary "$body"
```

Create aliases separately for providers with different secrets. The receiver
accepts payloads up to 256 KB and applies a global 100 requests/second rate with
a 200-request burst. Configure providers to retry non-2xx responses.

## Notification behavior

Normal fingerprints accumulate and send in one digest after the configured
interval, outside quiet hours. High fingerprints notify immediately, including
during quiet hours, and remain marked until acknowledged. Record-only
fingerprints stay in the ledger but do not notify. A Slack-compatible webhook is
any HTTPS endpoint accepting `{ "text": "…" }`.

## Deployment and data

The root `Dockerfile` builds both Vite and Rust in separate stages, runs as a
non-root user, serves the frontend and API on port 8080, and persists SQLite at
`/app/data`. Deployment, DNS, TLS, backups, and reverse-proxy trust remain the
operator's responsibility. The health endpoint is `GET /health`.

See [.factory/design.md](.factory/design.md) for the visual system and generated
asset provenance. Privacy and terms are available in-product at `/privacy` and
`/terms`.

## License

MIT — see [LICENSE](LICENSE).
