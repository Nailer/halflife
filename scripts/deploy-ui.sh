#!/usr/bin/env bash
# Rebuild state from the registry and ship the control room.
#
# State is regenerated before the build so the deployed site can never drift
# from what the CLI currently believes.
set -euo pipefail
cd "$(dirname "$0")/.."

./target/release/halflife export --out apps/control-room/public/state.json
./target/release/halflife export --operator --out apps/control-room/public/state.operator.json

# Deploy from the project directory, not from dist/: dist is wiped on every
# build, and a .vercel link living there gets destroyed with it -- which is how
# a stray "dist" project got created once already.
cd apps/control-room
vercel deploy --prod --yes --scope pleasants-projects
