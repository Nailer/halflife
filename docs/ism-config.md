# Interchain Security Module configuration

The destination's ISM *is* its security. This records the choice, what it costs
an attacker, and what the contract will not let an operator do.

## What the contract refuses

A Hyperlane recipient returning `address(0)` from `interchainSecurityModule()`
inherits the mailbox default. `HalflifeDestinationRegistry` cannot:

| Attempted configuration | Result |
|---|---|
| `address(0)` — inherit the default | `IsmRequired()` |
| `Types.NULL` — verify nothing, trust the relayer | `NullIsmRefused()` |
| Later downgrade to either | same reverts |
| Non-owner change | `NotOwner()` |

At the time of the $292M KelpDAO loss, **47% of deployed integrations on a
comparable system were running a single-verifier configuration.** Nobody chose
that; they accepted a default. So the weak configuration is made
unrepresentable rather than discouraged — a `revert`, not a comment.

### What the contract cannot do

It rejects the *empty* choice. It cannot tell a 1-of-1 multisig from a 7-of-10,
because `moduleType()` reports a category and not a threshold. That remains an
operator decision, and this document is where it is recorded rather than
assumed.

## The configuration for this deployment

Aggregation over two independent modules, both required:

```
AggregationISM (2 of 2)
├── MessageIdMultisigISM   Hyperlane's validator set for the Solana origin
└── MerkleRootMultisigISM  independent root attestation over the same message
```

Factory on Base Sepolia: `0x275aCcCa81cAD931dC6fB6E49ED233Bc99Bed4A7`
(`staticAggregationIsmFactory`).

### Cost to defeat

Both modules must be subverted for one message. Compromising a single validator
set is insufficient, which is the property a 1-of-1 lacks and the reason
aggregation was chosen over a bare multisig.

### Why suppression is not on this list

A relayer can censor but cannot forge. Withholding a Halflife message produces
staleness, and staleness blocks — so suppression yields the safe outcome and
needs no ISM countermeasure. That is deliberate: the ISM defends against forged
state, and expiry defends against absent state. Neither covers the other.

## Addresses

### Base Sepolia (destination, domain `84532`)

| | |
|---|---|
| Mailbox | `0x6966b0E55883d49BFB24539356a2f8A673E02039` |
| Aggregation ISM factory | `0x275aCcCa81cAD931dC6fB6E49ED233Bc99Bed4A7` |
| IGP | `0x28B02B97a850872C4D33C3E024fab6499ad96564` |
| Merkle tree hook | `0x86fb9F1c124fB20ff130C41a79a432F770f67AFD` |
| RPC | `https://sepolia.base.org` |

### Solana devnet (origin)

| | |
|---|---|
| Mailbox | `5yM5YrrzHCrp4ZPLKN9Y2eUAqEWsTbBqaorgbngQcR54` |
| Multisig ISM (message id) | `4Q1eZAovdqhihCCZyabX4u5rCJPq5R8YgyKVDCdjzTNV` |
| IGP | `9N7WmRUVL6b8agkS2GMephafHpwMWKAKAyjoNyW3j4Fi` |
