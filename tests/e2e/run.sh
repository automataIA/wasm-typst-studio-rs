#!/usr/bin/env bash
# Build + serve Typst Studio on :1420, run the Playwright smoke suite, tear down.
# Dev tooling only — not wired into CI. Requires node + a Trunk-built app.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

PORT=1420
BASE_URL="http://127.0.0.1:${PORT}"

# Ensure Playwright + a chromium build are available (first run downloads it).
if [ ! -d node_modules/playwright ]; then
  echo "Installing playwright (dev dependency)…"
  npm install --no-save playwright
fi
npx playwright install chromium

echo "Building (debug)…"
trunk build

echo "Serving on :${PORT}…"
trunk serve --port "${PORT}" >/tmp/trunk-e2e.log 2>&1 &
SERVER_PID=$!
trap 'kill "${SERVER_PID}" 2>/dev/null || true' EXIT

# Wait for the server to answer.
for _ in $(seq 1 60); do
  if curl -sf "${BASE_URL}" >/dev/null 2>&1; then break; fi
  sleep 1
done

echo "Running smoke suite…"
BASE_URL="${BASE_URL}" node tests/e2e/smoke.mjs
