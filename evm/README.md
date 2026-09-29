# EVM destination

Receives passport state from the canonical Solana registry over Hyperlane, and
serves it to consumers on this chain.

```bash
cd evm && forge install foundry-rs/forge-std --no-git && forge test
```

## What this side trusts

It does **not** verify the issuer's ed25519 signature — EVM has no ed25519
precompile, and doing it in Solidity costs more than the check is worth. The
chain of trust is:

```
issuer signature -> verified on Solana by the passport program
                 -> Solana consensus makes that state canonical
                 -> Hyperlane's ISM attests to what Solana said
                 -> this contract
```

Hyperlane authenticates *transport*: it establishes that this is what the Solana
registry says, not that the issuer's claim is independently true. Solana did
that part. A destination chain inherits Solana's verification, and that is a
real dependency rather than a detail. Re-verifying ed25519 here is the upgrade
path, not a claim made today.

## Why expiry is not enforced on receipt

`handle` stores whatever arrives; staleness is resolved at *read* time against
the reader's clock. That is what makes suppression useless — if no update ever
arrives the stored passport ages out and consumers block. Rejecting expired
messages on receipt would leave the last good state in place instead, which is
the failure this design exists to prevent.

`test_blocksWhenNoUpdateEverArrives` is the test that pins it.

## Conformance

`test/Conformance.t.sol` decodes bytes produced by the **Rust** encoder, taken
from `fixtures/vectors.json` and already verified by an independent Node
implementation. The other test file builds cores in Solidity and decodes them in
Solidity, which proves only that the contract agrees with itself.
