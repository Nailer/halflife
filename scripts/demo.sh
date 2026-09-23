#!/usr/bin/env bash
# Halflife v0.1 — the deterministic path, end to end.
set -euo pipefail
cd "$(dirname "$0")/.."
H=./target/release/halflife
OFFLINE=${OFFLINE:-}
[ -n "$OFFLINE" ] && FLAG="--offline" || FLAG=""

rule() { printf '\n\033[1m── %s\033[0m\n' "$1"; }

cargo build --release --quiet
[ -f issuer.json ] || $H keygen --out issuer.json

rule "1. Vulnerable closure — halo2_gadgets 0.4.0"
$H scan fixtures/orchard-vulnerable $FLAG --out out/vulnerable

rule "2. Patched closure — halo2_gadgets 0.5.0"
$H scan fixtures/orchard-patched $FLAG --out out/patched

rule "3. Consumer check: patched"
$H verify out/patched/passport.json --evidence out/patched/evidence.json \
  && echo "ACCEPT"

rule "4. Consumer check: vulnerable"
$H verify out/vulnerable/passport.json --evidence out/vulnerable/evidence.json \
  || echo "BLOCK"

rule "5. Fail-safe: no update ever arrives"
EXP=$(python3 -c "import json;print(json.load(open('out/patched/passport.json'))['core']['expires_at'])")
$H verify out/patched/passport.json --at $((EXP + 1)) || echo "BLOCK — stale, not trusted"

rule "6. Tamper: forge VALID onto the vulnerable passport"
python3 -c "
import json
p=json.load(open('out/vulnerable/passport.json'))
p['core']['status']='VALID'; p['core']['advisory_count']=0
json.dump(p,open('out/forged.json','w'))"
$H verify out/forged.json || echo "BLOCK — signature does not verify"
