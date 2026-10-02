#!/usr/bin/env bash
# The full chain, end to end: a passport written on Solana devnet drives a
# consumer on an EVM chain.
#
# The origin leg is real. The passport is read back from the deployed Solana
# registry and its canonical 125 bytes are delivered unchanged to an EVM
# destination, where a consumer contract decides whether to proceed.
#
# What is local: the chain and the mailbox. A faucet-funded testnet would swap
# the RPC and the mailbox address and nothing else -- the bytes, the contracts
# and the decision are identical. That boundary is stated rather than blurred.
set -euo pipefail
cd "$(dirname "$0")/.."

RPC=${EVM_RPC:-http://127.0.0.1:8545}
# anvil's first well-known account; a local chain, never a funded one.
KEY=${EVM_KEY:-0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80}
SOLANA_DOMAIN=1399811149

bold() { printf '\n\033[1m%s\033[0m\n' "$1"; }
ok()   { printf '  \033[32m✓\033[0m %s\n' "$1"; }
info() { printf '  \033[2m%s\033[0m\n' "$1"; }

cleanup() { [ -n "${ANVIL_PID:-}" ] && kill "$ANVIL_PID" 2>/dev/null || true; }
trap cleanup EXIT

if [ "${1:-}" = "--fresh" ]; then
  shift
  bold "0. Publish a fresh passport on devnet first"
  ./target/release/halflife-exercise run dependency-compromise >/dev/null
  ok "new exercise recorded"
fi

bold "1. Read a passport from Solana devnet"
CORE=$(./target/release/halflife-exercise fetch "$@" 2>/tmp/cc-meta)
sed 's/^/  /' /tmp/cc-meta
ok "fetched ${#CORE} hex chars from the deployed registry"

bold "2. Start a local EVM chain"
if ! curl -s -o /dev/null --max-time 2 -X POST "$RPC" \
     -H 'Content-Type: application/json' \
     -d '{"jsonrpc":"2.0","method":"eth_chainId","id":1}'; then
  anvil --silent --port 8545 &
  ANVIL_PID=$!
  until curl -s -o /dev/null --max-time 1 -X POST "$RPC" -H 'Content-Type: application/json' \
        -d '{"jsonrpc":"2.0","method":"eth_chainId","id":1}'; do sleep 0.4; done
  ok "anvil up on 8545"
else
  ok "using the chain already on $RPC"
fi

cd evm
dep() { forge create --rpc-url "$RPC" --private-key "$KEY" --broadcast "$@" 2>/dev/null \
          | grep "Deployed to:" | awk '{print $3}'; }

bold "3. Deploy the destination"
ISM=$(dep src/MockMailbox.sol:LocalAggregationIsm)
ok "aggregation ISM      $ISM"
MAILBOX=$(dep src/MockMailbox.sol:MockMailbox)
ok "mailbox              $MAILBOX"

# The Solana registry program as bytes32. Base58-DECODED, not hex-encoded: a
# Solana address is 32 bytes written in base58, and Hyperlane wants those bytes.
SENDER=0x$(python3 -c "
import sys
A='123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz'
n=0
for ch in 'CkDhRfJRiGEa3kgnEUEvCBgyht62MTkDD6e754DLtB2':
    n = n*58 + A.index(ch)
print(n.to_bytes(32,'big').hex())")
REG=$(dep src/HalflifeDestinationRegistry.sol:HalflifeDestinationRegistry \
       --constructor-args "$MAILBOX" "$SOLANA_DOMAIN" "$SENDER" "$ISM")
ok "destination registry $REG"

CIRCUIT=0x${CORE:4:64}
ISSUER=0x${CORE:68:64}
CONSUMER=$(dep src/HalflifeConsumer.sol:HalflifeConsumer \
            --constructor-args "$REG" "$CIRCUIT" "$ISSUER" 1)
ok "consumer             $CONSUMER"

bold "4. Before delivery — absence is not permission"
if cast call "$CONSUMER" "wouldProceed()(bool)" --rpc-url "$RPC" | grep -q true; then
  echo "  !! consumer would proceed with no passport"; exit 1
fi
ok "consumer refuses: no passport has ever arrived"

bold "5. Deliver the Solana bytes, unchanged"
cast send "$MAILBOX" "deliver(address,uint32,bytes32,bytes)" \
  "$REG" "$SOLANA_DOMAIN" "$SENDER" "$CORE" \
  --rpc-url "$RPC" --private-key "$KEY" >/dev/null
ok "handle() accepted 125 bytes written on Solana devnet"

STATUS=$(cast call "$REG" "statusNow(bytes32,bytes32)(uint8)" "$CIRCUIT" "$ISSUER" --rpc-url "$RPC")
case "$STATUS" in
  1) LABEL="VALID"   ;;
  2) LABEL="STALE"   ;;
  3) LABEL="INVALID" ;;
  *) LABEL="MISSING" ;;
esac
info "destination resolves the passport as $LABEL"
case "$LABEL" in
  STALE)   info "the passport's own expiry has passed — it aged out, nothing was published against it" ;;
  INVALID) info "an issuer reported this circuit should no longer be trusted" ;;
  MISSING) info "no passport for this (circuit, issuer) pair" ;;
esac

bold "6. The consumer decides"
if cast call "$CONSUMER" "wouldProceed()(bool)" --rpc-url "$RPC" | grep -q true; then
  ok "consumer PROCEEDS — evidence is current"
else
  ok "consumer BLOCKS — $LABEL"
fi

bold "Result"
cat <<TXT
  A passport signed off-chain, verified and stored by a Solana program on
  devnet, read back, and decoded by an EVM contract that reached the same
  conclusion from the same 125 bytes.

  Real: the signature, the Solana verification, the stored account, the bytes,
  both contracts, and the decision.
  Local: the chain and the mailbox. A funded testnet swaps the RPC and the
  mailbox address; nothing else changes.
TXT
