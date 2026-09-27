#!/usr/bin/env node
// Independent implementation of the Halflife passport wire format.
//
// Written from docs/passport-spec.md, not ported from the Rust. Zero
// dependencies — ed25519 comes from Node's own crypto — so a pass here means
// the *specification* is reproducible, not that two copies of one library
// agree with each other.
//
//   node packages/passport-ts/verify-vectors.mjs [fixtures/vectors.json]

import { readFileSync } from "node:fs";
import { createPrivateKey, createPublicKey, sign, verify } from "node:crypto";

const SIGNING_DOMAIN = Buffer.from("halflife-passport-v1", "utf8");
const CORE_LEN = 125;

const CAPABILITY = { C0: 0, C1: 1, C2: 2, C3: 3, C4: 4, C5: 5 };
const STATUS = { VALID: 1, INVALID: 2 };

// Node wants DER; these are the fixed Ed25519 wrappers around a raw key.
const PKCS8_PREFIX = Buffer.from("302e020100300506032b657004220420", "hex");
const SPKI_PREFIX = Buffer.from("302a300506032b6570032100", "hex");

const privFromSeed = (seed) =>
  createPrivateKey({
    key: Buffer.concat([PKCS8_PREFIX, seed]),
    format: "der",
    type: "pkcs8",
  });

const pubFromRaw = (raw) =>
  createPublicKey({
    key: Buffer.concat([SPKI_PREFIX, raw]),
    format: "der",
    type: "spki",
  });

/** Fixed-width little-endian encoding, per the spec's offset table. */
function canonicalBytes(core) {
  const out = Buffer.alloc(CORE_LEN);
  let i = 0;

  const u8 = (v) => { out.writeUInt8(v, i); i += 1; };
  const raw = (hex, len) => {
    const b = Buffer.from(hex, "hex");
    if (b.length !== len) throw new Error(`expected ${len} bytes, got ${b.length}`);
    b.copy(out, i); i += len;
  };
  // 64-bit fields arrive as strings. BigInt(string) is exact; BigInt(number)
  // would already have lost the value before we got here.
  const big = (v) => {
    if (typeof v !== "string") throw new Error("64-bit fields must be JSON strings");
    return BigInt(v);
  };
  const u64 = (v) => { out.writeBigUInt64LE(big(v), i); i += 8; };
  const i64 = (v) => { out.writeBigInt64LE(big(v), i); i += 8; };
  const u16 = (v) => { out.writeUInt16LE(v, i); i += 2; };

  const cap = CAPABILITY[core.capability];
  const st = STATUS[core.status];
  if (cap === undefined) throw new Error(`unknown capability ${core.capability}`);
  if (st === undefined) throw new Error(`unknown status ${core.status}`);

  u8(core.version);
  raw(core.circuit_hash, 32);
  raw(core.issuer, 32);
  u64(core.sequence);
  u8(cap);
  u8(st);
  i64(core.issued_at);
  i64(core.expires_at);
  raw(core.evidence_hash, 32);
  u16(core.advisory_count);

  if (i !== CORE_LEN) throw new Error(`wrote ${i} bytes, expected ${CORE_LEN}`);
  return out;
}

const preimage = (core) => Buffer.concat([SIGNING_DOMAIN, canonicalBytes(core)]);

function main() {
  const path = process.argv[2] ?? "fixtures/vectors.json";
  const file = JSON.parse(readFileSync(path, "utf8"));

  if (file.core_len !== CORE_LEN) {
    throw new Error(`core length disagrees: file ${file.core_len}, this impl ${CORE_LEN}`);
  }
  if (file.signing_domain !== SIGNING_DOMAIN.toString("utf8")) {
    throw new Error("signing domain disagrees");
  }

  const seed = Buffer.from(file.test_seed_hex, "hex");
  const priv = privFromSeed(seed);
  const pub = pubFromRaw(Buffer.from(file.issuer_pubkey_hex, "hex"));

  let failures = 0;
  for (const v of file.vectors) {
    try {
      const canonical = canonicalBytes(v.core).toString("hex");
      if (canonical !== v.canonical_hex) {
        throw new Error(`canonical differs\n    expected ${v.canonical_hex}\n    got      ${canonical}`);
      }

      const pre = preimage(v.core);
      if (pre.toString("hex") !== v.preimage_hex) throw new Error("preimage differs");

      // Both directions: re-sign must match byte-for-byte (ed25519 is
      // deterministic), and the recorded signature must verify.
      const resigned = sign(null, pre, priv).toString("hex");
      if (resigned !== v.signature_hex) throw new Error("signature differs on re-signing");

      if (!verify(null, pre, pub, Buffer.from(v.signature_hex, "hex"))) {
        throw new Error("recorded signature does not verify");
      }

      console.log(`  ok  ${v.name}`);
    } catch (e) {
      failures++;
      console.error(`  FAIL ${v.name}: ${e.message}`);
    }
  }

  if (failures > 0) {
    console.error(`\n${failures} vector(s) failed — the format is not portable.`);
    process.exit(1);
  }
  console.log(
    `\n${file.vectors.length} vectors reproduced byte-for-byte by an independent implementation.`
  );
}

main();
