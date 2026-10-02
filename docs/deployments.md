# Deployments

## Control room

**https://halflife-control-room.vercel.app**

Static build, redeployed with `./scripts/deploy-ui.sh`, which regenerates both
audience exports from the live registry before shipping. The `PUBLIC` /
`OPERATOR` toggle in the top right loads different exports — embargoed findings
are absent from the public one, not redacted.

## Solana devnet

| Program | Address |
|---|---|
| `halflife_passport` | [`CkDhRfJRiGEa3kgnEUEvCBgyht62MTkDD6e754DLtB2`](https://explorer.solana.com/address/CkDhRfJRiGEa3kgnEUEvCBgyht62MTkDD6e754DLtB2?cluster=devnet) |
| `halflife_consumer` | [`BAcrrJYj5Y5DfcqnHgDwm5rhJUJdowZh25NvFLJAUzSW`](https://explorer.solana.com/address/BAcrrJYj5Y5DfcqnHgDwm5rhJUJdowZh25NvFLJAUzSW?cluster=devnet) |

Upgrade authority: `Azh9zR13hCQiDYuSWZxJNmxarLzdr7uZrfohtUwMVPsh`

A single-key upgrade authority is a known gap, not a design. It moves to a
Squads multisig in B4.5 follow-up and to threshold control in B8.2. Until then,
whoever holds that key can replace the program a consumer is trusting — which is
precisely the deployment-drift problem P4B exists to detect.

### Reproduce

```bash
cd solana && anchor build
solana program deploy solana/target/deploy/halflife_passport.so \
  --program-id solana/target/deploy/halflife_passport-keypair.json \
  --keypair solana/deploy-keypair.json --url devnet
```

The `.so` artifacts are not reproducible byte-for-byte yet; that is B8.3.
Until then, "the deployed program matches this source" is a claim we ask you to
take on trust, and we would rather say so than imply otherwise.

## EVM destination

Not yet deployed. The contracts and tests are in `evm/`; a live deployment
requires a Hyperlane mailbox on the target chain and is tracked as B5.2/B5.3.

## Hyperlane (Sealevel)

Addresses taken from `hyperlane-monorepo/rust/sealevel/environments/`, verified
deployed on devnet.

| | Solana devnet |
|---|---|
| Mailbox | `5yM5YrrzHCrp4ZPLKN9Y2eUAqEWsTbBqaorgbngQcR54` |
| IGP program | `9N7WmRUVL6b8agkS2GMephafHpwMWKAKAyjoNyW3j4Fi` |
| Multisig ISM (message id) | `4Q1eZAovdqhihCCZyabX4u5rCJPq5R8YgyKVDCdjzTNV` |
| Validator announce | `7TfjfmTCuZVSq9mour8obQz2pXGNiEz29EqEEDnoexQ9` |

Solana testnet (`solanatestnet`) mailbox, for reference:
`75HBBLae3ddeneJVrZeyrDfv6vb7SMC3aCpBucSXS5aR`

### Why the dispatch is a CPI and not an off-chain relay

Hyperlane's mailbox requires the dispatching program to sign with a PDA derived
from `[b"hyperlane_dispatcher", b"-", b"dispatch_authority"]` under the declared
`sender`. From the mailbox source:

> a program uses a dispatch authority PDA to sign the CPI on its behalf.
> Instruction processing logic prevents a program from specifying any message
> sender it wants by requiring the relevant dispatch authority to sign the CPI.

That is precisely why the destination's `originSender` check carries weight. If
an off-chain process dispatched instead, the sender would be that process and the
destination would be trusting it rather than the registry — the trust chain in
`evm/README.md` would break at its first link. Anyone may pay to relay a
passport; nobody may invent what is relayed.

## Cross-chain

`./scripts/cross-chain.sh` runs the whole path: a passport is read back from the
deployed Solana registry on devnet and its canonical 125 bytes are delivered
unchanged to an EVM destination, where a consumer contract reaches a decision
from them.

```
./scripts/cross-chain.sh            # use the most recent passport
./scripts/cross-chain.sh --fresh    # publish a new one on devnet first
```

**Real:** the off-chain signature, the Solana verification, the stored account,
the bytes, both EVM contracts, and the decision.

**Local:** the chain and the mailbox. A faucet-funded testnet swaps the RPC and
the mailbox address and changes nothing else — the bytes, the contracts and the
decision are identical.

The mock mailbox is deliberately honest about its scope: it performs the one
check the real mailbox performs before calling a recipient — ask which ISM the
recipient requires and run that module's `verify` — so the recipient-side path
is exercised exactly as in production. It does **not** stand in for the
validator set, the relayer, or the aggregation of real attestations. Those live
off-chain and a local chain cannot replace them. It exercises the delivery path,
not the security of delivery.
