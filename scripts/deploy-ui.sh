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

# New Vercel projects on this account default to SSO deployment protection,
# which returns 403 to anyone not signed in -- fatal for a public demo and
# invisible until someone tries it logged out. Assert the site is reachable
# anonymously rather than assuming it.
echo
for path in / /state.json /state.operator.json; do
  code=$(curl -s -o /dev/null -w "%{http_code}" "https://halflife-control-room.vercel.app${path}")
  printf "  %-22s HTTP %s\n" "$path" "$code"
  if [ "$code" != "200" ]; then
    echo "  !! not publicly reachable -- check Deployment Protection in project settings"
    exit 1
  fi
done
echo "  public access verified"
