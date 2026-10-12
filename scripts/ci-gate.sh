#!/usr/bin/env bash
# usage: ci-gate.sh <pass|fail> <target>
#
# Runs the advisory gate against a target and checks it behaved as expected.
# Uses the pinned advisory snapshot so CI never depends on a network round trip.
set -euo pipefail
cd "$(dirname "$0")/.."

expect=$1
target=$2
H=./target/release/halflife
KEY=/tmp/ci-issuer.json
[ -f "$KEY" ] || $H keygen --out "$KEY" >/dev/null

if $H scan "$target" --issuer "$KEY" --offline --out "/tmp/gate-$(basename "$target")" --deny; then
  got=pass
else
  got=fail
fi

if [ "$got" != "$expect" ]; then
  echo "::error title=Gate misbehaved::$target should $expect but the gate said $got"
  exit 1
fi
echo "gate: $target -> $got (expected)"
