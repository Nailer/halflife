# Security notes

Findings from reviewing our own on-chain program, what was wrong, how it was
fixed, and what still is not.

## Found in self-review, 7 October 2026

Both holes were in `halflife-passport`, were present in the version deployed to
devnet at the time, and were found while preparing the first real Hyperlane
dispatch. Neither was exploited; the program holds no value and only we had
published against it.

### 1. Anyone could set a dispatch route

**What was wrong.** `set_route` used `init_if_needed` and checked nothing about
who was calling, so any account could create or overwrite the route for any
destination domain, including the `mailbox` program the route points at.

**Why it mattered.** `dispatch` calls `invoke_signed` into whichever program the
route names, passing the registry's *dispatch-authority PDA* as a signer. Solana
carries signer privileges down a CPI chain, so a route pointing at an attacker's
program would let it forward that signature to the **real** Hyperlane mailbox
and dispatch arbitrary bytes with **our program as the sender**.

That defeats the one check the destination registry relies on. The destination
cannot verify ed25519 and trusts that `originSender` really is the registry.
"Anyone may pay to relay a passport; nobody may invent what is relayed" would
have been false.

**The fix, as two independent controls.**

- Routes are **admin-only**, through a `Config` account created by `init_config`.
- The route's mailbox must be on a **fixed allowlist of known Hyperlane
  mailboxes** in the program source. Checked when a route is set *and again* at
  dispatch, so a compromised admin key still cannot aim the signature at a
  program of its choosing.

### 2. Anyone could register someone else's issuer key

**What was wrong.** `register_issuer` recorded whoever called first as that
key's `authority`, with no proof they held the key. An issuer's public key is
public, so anyone could register it first and then revoke it, squatting an
identity they have no claim to and locking the real issuer out.

**The fix.** Registration now requires the issuer's own ed25519 signature over
`halflife-register-issuer-v1 || authority`, verified through the sigverify
precompile. Binding the authority into the message makes the proof
non-transferable: a signature produced for one registrant is useless to another.

## How the fixes are pinned

`cargo run --release -p halflife-bench` runs each attack with a keypair that has
no authority and requires the specific refusal:

| Attack | Required result |
|---|---|
| `set_route` by a non-admin | `NotAdmin` |
| `set_route` to an unknown mailbox, even as admin | `UnknownMailbox` |
| `init_config` a second time, to seize the admin role | refused |
| register a key you do not hold | `SignerMismatch` |
| register with no proof at all | `MissingSignatureInstruction` |
| replay another registrant's proof | `SignedMessageMismatch` |

`scripts/verify-all.sh` fails if any of these lines is missing.

## What is still not solved

Stated so nobody has to find it.

- **`init_config` is first-caller-wins.** Whoever calls it first after a
  deployment becomes admin, so it has to run in the same step as deployment. It
  did on devnet. A production deployment should instead bind initialisation to
  the program's upgrade authority, which we did not do because our local test
  harness loads programs without a ProgramData account and cannot exercise it.
- **The upgrade authority is a single key.** Whoever holds it can replace the
  program a consumer is trusting, which is exactly the deployment-drift problem
  passport artifact binding exists to detect. It should move to a multisig.
- **The allowlist is code.** Adding a mailbox, such as a new cluster, means
  upgrading the program. That is the point of it, and also a cost.
- **The admin can still redirect *where* messages go** (domain and recipient),
  though not *which program receives the signature*. Misrouted passports are
  public state and harmless in themselves, but it is a griefing surface.
- **No external audit.** Nobody outside this project has reviewed the program.
  The two findings above were found by us, which says something good about our
  process and nothing about what we have not found.

## Why this is in the repository

This project's argument is that undisclosed weak verification is the problem. A
security tool that quietly fixed holes in its own enforcement layer, and told no
one, would be demonstrating the opposite. Both were real, both were ours, and
both were closed before they could matter.
