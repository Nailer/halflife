#!/usr/bin/env python3
"""Build registry fixtures for the real fleet.

Every version and checksum here is genuine crates.io data. What is *derived* is
the resolution: we take each crate's published dependency requirement and pick
the highest released version satisfying it, which is what Cargo would do. That
is labelled in each fixture rather than passed off as the crate's own lockfile.

Run: python3 scripts/build-fleet.py
"""

import json
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
UA = "halflife-fleet-builder"

# The fleet. Four crates genuinely pinned below the halo2_gadgets fix, two
# healthy, plus our two fixtures already on disk. Small and real — see Gate C.
FLEET = [
    ("lyrion-orchard", "0.11.0"),
    ("valar-orchard", "0.12.0"),
    ("zk-psi-verifier", "0.1.0"),
    ("zkdoc_sdk", "0.0.1"),
    ("orchard", "0.15.5"),
    ("voting-crypto-deps", "0.2.3"),
]

# Only these matter for the advisory; pulling a full transitive closure would
# mean resolving the whole graph, which is P4A proper rather than Stage 2.
TRACKED = {"halo2_gadgets", "halo2_proofs", "orchard", "zcash_primitives"}


def api(path):
    out = subprocess.run(
        ["curl", "-sS", f"https://crates.io/api/v1/{path}", "-H", f"User-Agent: {UA}"],
        capture_output=True, text=True, check=True,
    ).stdout
    return json.loads(out)


def versions(crate):
    """Published versions, newest first, with checksum and yank status.

    Yanked versions are kept deliberately. Yanking blocks *new* resolutions but
    does not touch an existing Cargo.lock, so a committed lockfile can still pin
    a yanked, vulnerable version — which is precisely the exposure worth finding.
    """
    return [
        (v["num"], v["checksum"], bool(v.get("yanked")))
        for v in api(f"crates/{crate}")["versions"]
        if "-" not in v["num"]  # skip pre-releases
    ]


def parse(v):
    return tuple(int(x) for x in v.split(".")[:3])


def satisfies(version, req):
    """Cargo caret semantics, enough for the `^x.y.z` forms in this fleet.

    For 0.x, a caret cannot cross a minor version — which is exactly why these
    crates cannot reach the 0.5.0 fix without a manual bump.
    """
    m = re.match(r"^\^?(\d+)\.(\d+)(?:\.(\d+))?$", req.strip())
    if not m:
        return False
    rmaj, rmin = int(m.group(1)), int(m.group(2))
    rpat = int(m.group(3) or 0)
    maj, minor, patch = parse(version)
    if maj != rmaj:
        return False
    if rmaj == 0:
        # ^0.y.z  =>  >=0.y.z, <0.(y+1).0
        return minor == rmin and (minor, patch) >= (rmin, rpat)
    return (minor, patch) >= (rmin, rpat)


def resolve(crate, version):
    """Resolve the crate's tracked normal dependencies to concrete versions.

    Optional dependencies are included. A security tool should assume a feature
    may be enabled; excluding them would under-report. Each is labelled in the
    fixture so the assumption is visible rather than buried.
    """
    deps = api(f"crates/{crate}/{version}/dependencies")["dependencies"]
    picked, seen = [], set()
    for d in deps:
        if d["kind"] != "normal" or d["crate_id"] not in TRACKED:
            continue
        if d["crate_id"] in seen:
            continue
        cands = versions(d["crate_id"])
        match = next((c for c in cands if satisfies(c[0], d["req"])), None)
        if match is None:
            print(f"  ! {d['crate_id']} {d['req']}: no published version matches",
                  file=sys.stderr)
            continue
        seen.add(d["crate_id"])
        num, cksum, yanked = match
        picked.append({
            "name": d["crate_id"], "version": num, "checksum": cksum,
            "req": d["req"], "yanked": yanked, "optional": bool(d.get("optional")),
        })
    return picked


def write_fixture(crate, version, resolved):
    slug = crate.replace("_", "-")
    d = ROOT / "fixtures" / "fleet" / slug
    d.mkdir(parents=True, exist_ok=True)

    lines = [
        f"# Halflife fleet fixture for {crate} {version}.",
        "#",
        "# Versions and checksums are genuine crates.io data. The resolution is",
        "# DERIVED: each published requirement is resolved to the highest release",
        "# satisfying it, as Cargo would. This is not the crate's own lockfile.",
        "#",
        "# Optional dependencies are included, on the conservative reading that a",
        "# feature may be enabled. Yanked versions are kept: yanking blocks new",
        "# resolutions but leaves an existing lockfile pinning them untouched.",
        "version = 3",
        "",
        "[[package]]",
        f'name = "{crate}"',
        f'version = "{version}"',
        "dependencies = [",
        *[f'    "{r["name"]}",' for r in resolved],
        "]",
        "",
    ]
    for r in resolved:
        flags = []
        if r["yanked"]:
            flags.append("YANKED from crates.io")
        if r["optional"]:
            flags.append("optional/feature-gated")
        note = f"  ({'; '.join(flags)})" if flags else ""
        lines += [
            f"# declared requirement: {r['req']}{note}",
            "[[package]]",
            f'name = "{r["name"]}"',
            f'version = "{r["version"]}"',
            'source = "registry+https://github.com/rust-lang/crates.io-index"',
            f'checksum = "{r["checksum"]}"',
            "",
        ]
    (d / "Cargo.lock").write_text("\n".join(lines))
    (d / "halflife.toml").write_text(
        "[subject]\n"
        f'name = "{crate}"\n'
        f'repository = "https://crates.io/crates/{crate}"\n'
        f'commit = "crates.io:{crate}@{version}"\n'
        'proof_system = "halo2"\n'
    )
    return d


def main():
    print(f"building fleet fixtures under {ROOT / 'fixtures' / 'fleet'}\n")
    for crate, version in FLEET:
        resolved = resolve(crate, version)
        write_fixture(crate, version, resolved)
        detail = "  ".join(
            f"{r['name']}@{r['version']}" + ("[yanked]" if r["yanked"] else "")
            for r in resolved
        )
        print(f"  {crate:<22} {version:<9} {detail}")
    print(f"\n{len(FLEET)} fixtures written")


if __name__ == "__main__":
    main()
