#!/usr/bin/env bash
# Halflife — full verification pass.
#
# One command that exercises every layer and fails loudly. Every number this
# prints is measured here, not quoted from elsewhere.
#
#   ./scripts/verify-all.sh
#
# Requires: cargo, node, forge, anchor, solana. Network only for the devnet
# check at the end, which is skipped when offline.
set -uo pipefail
cd "$(dirname "$0")/.."

PASS=0; FAIL=0; SKIP=0
H=./target/release/halflife

bold() { printf '\n\033[1m%s\033[0m\n'; }
step() { printf '\n\033[1m── %s\033[0m\n' "$1"; }
ok()   { printf '   \033[32m✓\033[0m %s\n' "$1"; PASS=$((PASS+1)); }
bad()  { printf '   \033[31m✗\033[0m %s\n' "$1"; FAIL=$((FAIL+1)); }
skip() { printf '   \033[33m–\033[0m %s\n' "$1"; SKIP=$((SKIP+1)); }
run()  { if eval "$2" >/tmp/hl.out 2>&1; then ok "$1"; else bad "$1"; sed 's/^/       /' /tmp/hl.out | tail -12; fi }

printf '\033[1mHALFLIFE — FULL VERIFICATION\033[0m\n'
printf 'commit %s · %s\n' "$(git rev-parse --short HEAD)" "$(date -u +%Y-%m-%dT%H:%MZ)"

# ---------------------------------------------------------------- 1. build
step "1. Build"
run "host workspace compiles" "cargo build --release --quiet"

# ---------------------------------------------------------------- 2. tests
step "2. Unit tests"
if cargo test --release --quiet >/tmp/hl.t 2>&1; then
  n=$(grep -oE '[0-9]+ passed' /tmp/hl.t | awk '{s+=$1} END {print s}')
  ok "cargo test — ${n:-0} assertions passed"
else
  bad "cargo test"; tail -20 /tmp/hl.t | sed 's/^/       /'
fi

# ------------------------------------------------- 3. format conformance
step "3. Wire format — three independent implementations"
run "Rust reproduces all vectors"  "$H vectors verify fixtures/vectors.json"
run "Node reproduces all vectors"  "node packages/passport-ts/verify-vectors.mjs fixtures/vectors.json"
if (cd evm && forge test --match-contract ConformanceTest >/tmp/hl.sol 2>&1); then
  ok "Solidity decodes Rust-produced vectors"
else
  bad "Solidity conformance"; tail -12 /tmp/hl.sol | sed 's/^/       /'
fi

# Negative control: the suite must actually catch a tampered vector.
python3 -c "
import json; d=json.load(open('fixtures/vectors.json'))
d['vectors'][0]['core']['sequence']='424242'
json.dump(d, open('/tmp/hl-tampered.json','w'))"
if node packages/passport-ts/verify-vectors.mjs /tmp/hl-tampered.json >/dev/null 2>&1; then
  bad "NEGATIVE CONTROL: tampered vector was accepted"
else
  ok "tampered vector rejected (negative control)"
fi

# ------------------------------------------------------- 4. passport flow
step "4. Passport issuance and the fail-safe"
rm -rf /tmp/hl-out /tmp/hl-issuer.json
run "keygen" "$H keygen --out /tmp/hl-issuer.json"
run "scan vulnerable closure → INVALID" \
  "$H scan fixtures/orchard-vulnerable --issuer /tmp/hl-issuer.json --offline --out /tmp/hl-out/vuln && grep -q '\"INVALID\"' /tmp/hl-out/vuln/passport.json"
run "scan patched closure → VALID" \
  "$H scan fixtures/orchard-patched --issuer /tmp/hl-issuer.json --offline --out /tmp/hl-out/ok && grep -q '\"VALID\"' /tmp/hl-out/ok/passport.json"
run "signature + evidence binding verify" \
  "$H verify /tmp/hl-out/ok/passport.json --evidence /tmp/hl-out/ok/evidence.json"
run "invalid passport blocks (exit 2)" \
  "! $H verify /tmp/hl-out/vuln/passport.json --evidence /tmp/hl-out/vuln/evidence.json"

EXP=$(python3 -c "import json;print(json.load(open('/tmp/hl-out/ok/passport.json'))['core']['expires_at'])")
run "FAIL-SAFE: expired passport → STALE, blocks" \
  "! $H verify /tmp/hl-out/ok/passport.json --at \$((EXP + 1))"

python3 -c "
import json; p=json.load(open('/tmp/hl-out/vuln/passport.json'))
p['core']['status']='VALID'; p['core']['advisory_count']=0
json.dump(p, open('/tmp/hl-forged.json','w'))"
run "forged VALID rejected" "! $H verify /tmp/hl-forged.json"
run "mismatched evidence rejected" \
  "! $H verify /tmp/hl-out/vuln/passport.json --evidence /tmp/hl-out/ok/evidence.json"

# --------------------------------------------------- 5. registry + lineage
step "5. Registry and the reverse index"
rm -f /tmp/hl.db
for f in fixtures/fleet/*/ fixtures/orchard-vulnerable fixtures/orchard-patched; do
  $H register "${f%/}" --issuer /tmp/hl-issuer.json --offline --disclosure public \
    --registry /tmp/hl.db >/dev/null 2>&1 || bad "register ${f}"
done
N=$($H registry list --registry /tmp/hl.db 2>/dev/null | grep -oE '^[0-9]+ registered' | awk '{print $1}')
[ "${N:-0}" -ge 8 ] && ok "fleet registered — $N systems" || bad "expected >=8 systems, got ${N:-0}"

# -------------------------------------------------------- 6. projection
step "6. Projection — affected vs clean, and the embargo"
AFF=$($H impact halo2_gadgets@0.4.0 --operator --registry /tmp/hl.db --json 2>/dev/null | python3 -c "import json,sys;print(len(json.load(sys.stdin)['affected']))" 2>/dev/null)
[ "${AFF:-0}" -ge 3 ] && ok "vulnerable dep → $AFF affected" || bad "expected >=3 affected, got ${AFF:-0}"

CLEAN=$($H impact halo2_gadgets@0.5.0 --operator --registry /tmp/hl.db --json 2>/dev/null | python3 -c "import json,sys;d=json.load(sys.stdin);print(len(d['affected']))" 2>/dev/null)
[ "${CLEAN:-x}" = "0" ] && ok "healthy dep → 0 affected (no false impact)" || bad "healthy dep reported ${CLEAN} affected"

# Embargo: register one circuit privately and confirm it is invisible publicly.
$H register fixtures/orchard-vulnerable --issuer /tmp/hl-issuer.json --offline \
  --disclosure embargoed --sequence 2 --registry /tmp/hl.db >/dev/null 2>&1
PUBA=$($H impact halo2_gadgets@0.4.0 --registry /tmp/hl.db --json 2>/dev/null | python3 -c "import json,sys;print(len(json.load(sys.stdin)['affected']))" 2>/dev/null)
OPA=$($H impact halo2_gadgets@0.4.0 --operator --registry /tmp/hl.db --json 2>/dev/null | python3 -c "import json,sys;print(len(json.load(sys.stdin)['affected']))" 2>/dev/null)
if [ "${PUBA:-0}" -lt "${OPA:-0}" ]; then
  ok "embargoed circuit hidden from public view ($PUBA public vs $OPA operator)"
else
  bad "embargo leak: public saw $PUBA, operator saw $OPA"
fi

# ------------------------------------------------------------- 7. Solana
step "7. Solana — enforcement and measured cost"
if [ -f solana/target/deploy/halflife_passport.so ]; then
  if cargo run --release -q -p halflife-bench >/tmp/hl.bench 2>&1; then
    CU=$(grep "consumer check" /tmp/hl.bench | grep -oE '[0-9]+ CU' | grep -oE '[0-9]+')
    ok "passport check measured at ${CU} CU"
    grep -qE "PassportInvalid"        /tmp/hl.bench && ok "kill switch fires (PassportInvalid)"        || bad "kill switch"
    grep -qE "SequenceNotIncreasing"  /tmp/hl.bench && ok "replay refused (SequenceNotIncreasing)"     || bad "replay guard"
    grep -qE "PassportStale"          /tmp/hl.bench && ok "FAIL-SAFE on-chain: stale blocks, nothing published" || bad "on-chain fail-safe"
    grep -qE "round trip preserves"   /tmp/hl.bench && ok "on-chain re-encoding reproduces the signed bytes"    || bad "round-trip integrity"
    # Each attack that used to land must be refused with its specific error.
    # Matching the name, not just "refused", so a different failure cannot pass.
    for pair in "non-admin:NotAdmin" "unknown mailbox:UnknownMailbox" \
                "key you don't hold:SignerMismatch" "no proof:MissingSignatureInstruction" \
                "replaying a proof:SignedMessageMismatch" "second time:refused"; do
      label="${pair%%:*}"; want="${pair##*:}"
      if grep -E "$label" /tmp/hl.bench | grep -q "$want"; then
        ok "access control: $label -> $want"
      else
        bad "access control: $label was not refused with $want"
      fi
    done
  else
    bad "benchmark"; tail -12 /tmp/hl.bench | sed 's/^/       /'
  fi
else
  skip "Solana programs not built — run: cd solana && anchor build"
fi

# ---------------------------------------------------------------- 8. EVM
step "8. EVM destination"
if [ -d evm/lib/forge-std ]; then
  if (cd evm && forge test >/tmp/hl.evm 2>&1); then
    n=$(grep -oE '[0-9]+ tests passed' /tmp/hl.evm | tail -1 | grep -oE '[0-9]+')
    ok "forge test — ${n:-?} tests passed"
    grep -q "test_blocksWhenNoUpdateEverArrives().*PASS\|\[PASS\] test_blocksWhenNoUpdateEverArrives" /tmp/hl.evm \
      && ok "FAIL-SAFE cross-chain: blocks when no update ever arrives" \
      || bad "cross-chain fail-safe test missing"
  else
    bad "forge test"; tail -15 /tmp/hl.evm | sed 's/^/       /'
  fi
else
  skip "forge-std missing — run: cd evm && forge install foundry-rs/forge-std --no-git"
fi

# ------------------------------------------------------ 8b. fire drill
step "8b. Fire Drill — recorded exercises"
latest_dep=$(ls -t exercises/dependency-compromise-*.json 2>/dev/null | head -1)
latest_cen=$(ls -t exercises/relayer-censorship-*.json 2>/dev/null | head -1)
if [ -n "$latest_dep" ] && [ -n "$latest_cen" ]; then
  run "dependency-compromise record verifies against devnet" \
      "./target/release/halflife-exercise verify $latest_dep"
  run "relayer-censorship record verifies against devnet" \
      "./target/release/halflife-exercise verify $latest_cen"
  # The censorship record must show the consumer blocking without any
  # invalidation having been published -- exactly one on-chain write.
  WRITES=$(python3 -c "
import json,sys
d=json.load(open('$latest_cen'))
print(sum(1 for e in d['events'] if e['evidence']['type']=='SOLANA_TRANSACTION'))")
  if [ "${WRITES:-0}" -eq 1 ]; then
    ok "censorship: consumer blocked after exactly 1 on-chain write (the baseline)"
  else
    bad "censorship: expected 1 on-chain write, record shows ${WRITES}"
  fi
else
  skip "no exercise records — run: halflife-exercise run dependency-compromise"
fi

# ------------------------------------------------ 8c. cross-chain + self
step "8c. Cross-chain and self-assurance"
if command -v anvil >/dev/null 2>&1 && [ -f evm/src/MockMailbox.sol ]; then
  if ./scripts/cross-chain.sh >/tmp/hl.cc 2>&1; then
    grep -q "consumer BLOCKS\|consumer PROCEEDS" /tmp/hl.cc \
      && ok "devnet passport drives an EVM consumer to a decision" \
      || bad "cross-chain produced no decision"
  else
    bad "cross-chain"; tail -10 /tmp/hl.cc | sed 's/^/       /'
  fi
else
  skip "anvil unavailable — cross-chain path not exercised"
fi

# The Hyperlane origin leg, against the real deployed devnet mailbox. This
# re-reads the mailbox's OWN account rather than trusting our recording.
disp=$(ls -t exercises/hyperlane-dispatch-*.json 2>/dev/null | head -1)
if [ -n "$disp" ]; then
  run "real Hyperlane dispatch verifies against devnet (origin leg)" \
      "./target/release/halflife-exercise verify-dispatch $disp"
else
  skip "no Hyperlane dispatch record — run: halflife-exercise dispatch"
fi

# Halflife under its own instrument. The result is allowed to be unflattering;
# what is not allowed is the scan failing to run.
if ./scripts/self-passport.sh /tmp/hl-self >/tmp/hl.self 2>&1; then
  st=$(python3 -c "import json;print(json.load(open('/tmp/hl-self/passport.json'))['core']['status'])")
  ok "Halflife issues a passport for itself — reports $st"
else
  bad "self-passport"; tail -8 /tmp/hl.self | sed 's/^/       /'
fi

# -------------------------------------------------------- 9. live devnet
step "9. Devnet deployments"
for pair in "halflife_passport:CkDhRfJRiGEa3kgnEUEvCBgyht62MTkDD6e754DLtB2" \
            "halflife_consumer:BAcrrJYj5Y5DfcqnHgDwm5rhJUJdowZh25NvFLJAUzSW"; do
  name="${pair%%:*}"; addr="${pair##*:}"
  if solana program show "$addr" --url devnet --keypair solana/deploy-keypair.json 2>/dev/null | grep -q "Program Id"; then
    ok "$name live at $addr"
  else
    skip "$name — devnet unreachable or not deployed"
  fi
done

# --------------------------------------------------------------- summary
printf '\n\033[1m────────────────────────────────────────\033[0m\n'
printf '  \033[32m%s passed\033[0m   \033[31m%s failed\033[0m   \033[33m%s skipped\033[0m\n' "$PASS" "$FAIL" "$SKIP"
if [ "$FAIL" -eq 0 ]; then
  printf '  \033[32mall checks green\033[0m\n\n'; exit 0
else
  printf '  \033[31mverification FAILED\033[0m\n\n'; exit 1
fi
