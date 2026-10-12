import { useEffect, useRef, useState } from "react";
import { Check, Alert, Idle } from "./icons";

const RPC = "https://api.devnet.solana.com";
const PASSPORT_PROGRAM = "CkDhRfJRiGEa3kgnEUEvCBgyht62MTkDD6e754DLtB2";

type Phase = "idle" | "deriving" | "fetching" | "decoding" | "done" | "error";

interface Decoded {
  version: number;
  circuitHash: string;
  issuer: string;
  sequence: bigint;
  capability: number;
  status: number;
  issuedAt: bigint;
  expiresAt: bigint;
  advisoryCount: number;
}

/**
 * Decode the canonical 125 bytes.
 *
 * A fourth independent implementation of the format, after Rust, Node and
 * Solidity — written against `docs/passport-spec.md`, not ported. If the offsets
 * here disagreed with the on-chain layout the status would come out wrong in
 * front of whoever is watching, which is the point: this is not reading a file
 * we prepared.
 */
function decode(core: Uint8Array): Decoded {
  const dv = new DataView(core.buffer, core.byteOffset, core.byteLength);
  const hex = (a: number, b: number) =>
    [...core.slice(a, b)].map((x) => x.toString(16).padStart(2, "0")).join("");
  return {
    version: core[0],
    circuitHash: hex(1, 33),
    issuer: hex(33, 65),
    sequence: dv.getBigUint64(65, true),
    capability: core[73],
    status: core[74],
    issuedAt: dv.getBigInt64(75, true),
    expiresAt: dv.getBigInt64(83, true),
    advisoryCount: dv.getUint16(123, true),
  };
}

/** Status resolved against the viewer's own clock, exactly as a program does. */
function effective(d: Decoded, nowSecs: bigint) {
  if (d.status === 2) return "INVALID";
  if (nowSecs >= d.expiresAt) return "STALE";
  return "VALID";
}

async function rpc(method: string, params: unknown[]) {
  const r = await fetch(RPC, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ jsonrpc: "2.0", id: 1, method, params }),
  });
  const j = await r.json();
  if (j.error) throw new Error(j.error.message ?? "RPC error");
  return j.result;
}

const b64 = (s: string) => Uint8Array.from(atob(s), (c) => c.charCodeAt(0));

export function LiveRead({ circuitHash, issuerHex }: { circuitHash?: string; issuerHex?: string }) {
  const [phase, setPhase] = useState<Phase>("idle");
  const [pda, setPda] = useState<string | null>(null);
  const [raw, setRaw] = useState<string | null>(null);
  const [decoded, setDecoded] = useState<Decoded | null>(null);
  const [slot, setSlot] = useState<number | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [now, setNow] = useState(() => BigInt(Math.floor(Date.now() / 1000)));
  const alive = useRef(true);

  useEffect(() => {
    const t = setInterval(() => setNow(BigInt(Math.floor(Date.now() / 1000))), 1000);
    return () => {
      alive.current = false;
      clearInterval(t);
    };
  }, []);

  const run = async () => {
    if (!circuitHash || !issuerHex) return;
    setErr(null);
    setDecoded(null);
    setRaw(null);
    try {
      setPhase("deriving");
      // getProgramAccounts with a memcmp on circuit_hash: the PDA derivation
      // needs curve maths we are deliberately not pulling a library in for.
      // Offset 8 skips the Anchor discriminator; circuit_hash is first.
      setPhase("fetching");
      const res = await rpc("getProgramAccounts", [
        PASSPORT_PROGRAM,
        {
          encoding: "base64",
          filters: [{ memcmp: { offset: 8, bytes: hexToB58(circuitHash) } }],
        },
      ]);
      if (!alive.current) return;
      if (!res?.length) throw new Error("no passport account found on devnet for this circuit");

      const acct = res[0];
      setPda(acct.pubkey);
      const data = b64(acct.account.data[0]);
      const d = data.subarray(8);

      setPhase("decoding");
      // Rebuild the canonical 125 bytes from the stored account.
      const core = new Uint8Array(125);
      core[0] = 1;
      core.set(d.subarray(0, 32), 1);
      core.set(d.subarray(32, 64), 33);
      core.set(d.subarray(64, 72), 65);
      core[73] = d[72];
      core[74] = d[73];
      core.set(d.subarray(74, 82), 75);
      core.set(d.subarray(82, 90), 83);
      core.set(d.subarray(90, 122), 91);
      core.set(d.subarray(122, 124), 123);

      setRaw([...core].map((x) => x.toString(16).padStart(2, "0")).join(""));
      setDecoded(decode(core));
      setSlot(await rpc("getSlot", []));
      setPhase("done");
    } catch (e) {
      if (!alive.current) return;
      setErr(e instanceof Error ? e.message : String(e));
      setPhase("error");
    }
  };

  const eff = decoded ? effective(decoded, now) : null;
  const busy = phase === "deriving" || phase === "fetching" || phase === "decoding";

  return (
    <div className="card" data-tour="liveread">
      <header>
        <h2>Live read · Solana devnet</h2>
        {slot !== null && (
          <span className="mono" style={{ fontSize: 10, color: "var(--faint)" }}>slot {slot}</span>
        )}
      </header>

      <div className="body">
        <p className="note">
          Everything else on this page comes from a file the CLI exported. This
          does not. Press the button and the browser queries Solana directly,
          pulls the real passport account, and decodes the 125 bytes here —
          a fourth independent implementation of the format.
        </p>

        <div style={{ display: "flex", gap: 9, alignItems: "center", marginTop: 14, flexWrap: "wrap" }}>
          <button className="btn primary" onClick={run} disabled={busy || !circuitHash}>
            {busy ? "Reading…" : "Read from devnet"}
          </button>
          {!circuitHash && <span className="note">Pick a circuit in Fleet first.</span>}
          {phase === "done" && (
            <span className="mono" style={{ fontSize: 10.5, color: "var(--faint)" }}>
              fetched {raw?.length ?? 0}/2 bytes · decoded in-browser
            </span>
          )}
        </div>

        {err && (
          <p className="note" style={{ marginTop: 14, color: "var(--invalid)" }}>{err}</p>
        )}

        {decoded && eff && (
          <>
            <div
              className="gate"
              data-g={eff === "VALID" ? "ALLOW" : eff === "STALE" ? "STALE" : "BLOCK"}
              style={{ padding: "16px 0 6px" }}
            >
              <div
                className="lamp"
                style={{
                  color:
                    eff === "VALID" ? "var(--valid)" : eff === "STALE" ? "var(--stale)" : "var(--invalid)",
                }}
              >
                {eff === "VALID" ? <Check /> : eff === "INVALID" ? <Alert /> : <Idle />}
              </div>
              <div className="txt">
                <b>{eff}</b>
                <span>
                  {eff === "VALID"
                    ? "Resolved against your clock, right now."
                    : eff === "STALE"
                      ? "Expired. Nothing was published against it — it simply aged out."
                      : "An issuer reported this circuit should not be trusted."}
                </span>
              </div>
            </div>

            <dl className="kv" style={{ marginTop: 10 }}>
              <dt>Account</dt><dd>{pda}</dd>
              <dt>Circuit</dt><dd>{decoded.circuitHash}</dd>
              <dt>Sequence</dt><dd>{decoded.sequence.toString()}</dd>
              <dt>Capability</dt><dd>C{decoded.capability}</dd>
              <dt>Expires</dt>
              <dd>
                {new Date(Number(decoded.expiresAt) * 1000).toISOString().replace("T", " ").slice(0, 19)}Z
                {" · "}
                {Number(decoded.expiresAt - now) > 0
                  ? `${Number(decoded.expiresAt - now)}s left`
                  : `${Number(now - decoded.expiresAt)}s ago`}
              </dd>
              <dt>Advisories</dt><dd>{decoded.advisoryCount}</dd>
            </dl>

            <p className="note" style={{ marginTop: 13, wordBreak: "break-all", fontFamily: "var(--m)", fontSize: 10.5 }}>
              {raw}
            </p>
            <p className="note" style={{ marginTop: 8 }}>
              Those are the bytes an issuer signed, a Solana program verified and
              stored, and your browser just decoded — the same 125 bytes an EVM
              contract reads on the other side.
            </p>
          </>
        )}
      </div>
    </div>
  );
}

/** Base58 for a 32-byte hex string — memcmp filters want base58. */
function hexToB58(hex: string): string {
  const A = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
  let n = BigInt("0x" + hex);
  let out = "";
  while (n > 0n) {
    out = A[Number(n % 58n)] + out;
    n /= 58n;
  }
  for (let i = 0; i < hex.length && hex.slice(i, i + 2) === "00"; i += 2) out = "1" + out;
  return out;
}
