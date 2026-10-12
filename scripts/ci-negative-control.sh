#!/usr/bin/env bash
# A conformance suite that cannot fail proves nothing. Corrupt one vector and
# confirm the independent implementation rejects it.
set -euo pipefail
cd "$(dirname "$0")/.."

python3 - <<'PY'
import json
d = json.load(open("fixtures/vectors.json"))
d["vectors"][0]["core"]["sequence"] = "424242"
json.dump(d, open("/tmp/tampered-vectors.json", "w"))
PY

if node packages/passport-ts/verify-vectors.mjs /tmp/tampered-vectors.json >/dev/null 2>&1; then
  echo "::error::a tampered vector was accepted, so the suite cannot fail"
  exit 1
fi
echo "tampered vector rejected"
