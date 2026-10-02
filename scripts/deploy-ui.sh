#!/usr/bin/env bash
# Rebuild state from the registry and ship the control room.
set -euo pipefail
cd "$(dirname "$0")/.."

./target/release/halflife export --out apps/control-room/public/state.json
./target/release/halflife export --operator --out apps/control-room/public/state.operator.json

cd apps/control-room
npm run build
# State files live in public/ for dev; Vite copies them, but copy again in case
# an export ran after the build.
cp public/state*.json dist/
vercel deploy --prod --yes --scope pleasants-projects dist
