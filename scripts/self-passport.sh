#!/usr/bin/env bash
# Halflife issues a passport for its own codebase, under the same rules it
# applies to everyone else.
#
# The point is not the result. It is that the system which asks other projects
# for provenance is subject to the same instrument, with no exemption — and
# that it reports its own real state, including an unflattering one.
set -euo pipefail
cd "$(dirname "$0")/.."

OUT=${1:-self}
ISSUER=${HALFLIFE_ISSUER:-issuer.json}
[ -f "$ISSUER" ] || ./target/release/halflife keygen --out "$ISSUER" >/dev/null

# Our own Cargo.lock is the closure. No special case, no curated subset.
./target/release/halflife scan . \
  --issuer "$ISSUER" --disclosure public --out "$OUT" "$@" 2>/dev/null \
  || ./target/release/halflife scan . --issuer "$ISSUER" --disclosure public --out "$OUT"

echo
echo "Halflife's own passport:"
python3 - "$OUT" <<'PY'
import json, sys
p = json.load(open(f"{sys.argv[1]}/passport.json"))
e = json.load(open(f"{sys.argv[1]}/evidence.json"))
c = p["core"]
print(f"  circuit     {c['circuit_hash']}")
print(f"  capability  C{ {'C0':0,'C1':1,'C2':2,'C3':3,'C4':4,'C5':5}[c['capability']] }")
print(f"  status      {c['status']}")
print(f"  closure     {len(e['dependencies'])} packages")
hits = [d for d in e["dependencies"] if d["advisories"]]
if hits:
    for d in hits:
        print(f"  finding     {d['name']}@{d['version']}  {', '.join(d['advisories'])}")
else:
    print("  findings    none")
print()
print("  Claimed at C1 only: a dependency closure resolved and matched against")
print("  published advisories. Nothing above that is asserted, here or anywhere.")
PY
