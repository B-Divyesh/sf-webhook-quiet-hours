#!/usr/bin/env bash
set -euo pipefail

e2e_dir="$(mktemp -d)"
cleanup() {
  rm -rf -- "$e2e_dir"
}
trap cleanup EXIT INT TERM

npm run build
PORT=4173 \
ADMIN_TOKEN=qa-token \
DATA_ENCRYPTION_KEY=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA= \
DATABASE_URL="sqlite://$e2e_dir/quiet-hours.db?mode=rwc" \
PUBLIC_URL=http://127.0.0.1:4173 \
DIST_DIR=dist \
RUST_LOG=error \
cargo run --quiet
