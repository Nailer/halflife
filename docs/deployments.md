# Deployments

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
