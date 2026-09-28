#!/usr/bin/env python3
"""Pin the offline advisory snapshot from live OSV.

The snapshot must be derived from what the fixtures actually contain, never
hand-picked. A hand-picked snapshot silently under-reports: an earlier version
of this file listed only halo2_gadgets 0.4.0, so fleet members pinning 0.2.0 and
0.3.1 came back clean while OSV reports every version below 0.5.0 as affected.

Run after changing any fixture: python3 scripts/build-snapshot.py
"""

import json
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
OUT = ROOT / "fixtures" / "osv-snapshot.json"

PKG = re.compile(
    r'\[\[package\]\]\s*\nname\s*=\s*"([^"]+)"\s*\nversion\s*=\s*"([^"]+)"([^\[]*)',
    re.MULTILINE,
)


def collect():
    """Every registry-sourced (name, version) across all fixture lockfiles."""
    found = set()
    for lock in sorted(ROOT.glob("fixtures/**/Cargo.lock")):
        text = lock.read_text()
        for name, version, tail in PKG.findall(text):
            # Only registry packages have advisories to match against.
            if "registry+" in tail:
                found.add((name, version))
    return sorted(found)


def query(pkgs):
    body = {
        "queries": [
            {"package": {"name": n, "ecosystem": "crates.io"}, "version": v}
            for n, v in pkgs
        ]
    }
    out = subprocess.run(
        ["curl", "-sS", "-X", "POST", "https://api.osv.dev/v1/querybatch",
         "-H", "Content-Type: application/json", "-d", json.dumps(body)],
        capture_output=True, text=True, check=True,
    ).stdout
    return json.loads(out).get("results", [])


def main():
    pkgs = collect()
    if not pkgs:
        print("no fixture packages found", file=sys.stderr)
        return 1
    print(f"querying OSV for {len(pkgs)} package versions from fixtures\n")

    snapshot = {}
    for (name, version), result in zip(pkgs, query(pkgs)):
        ids = sorted({v["id"] for v in (result or {}).get("vulns", [])})
        mark = "  <-- " + ", ".join(ids) if ids else ""
        print(f"  {name:<26} {version:<10}{mark}")
        if ids:
            snapshot[f"{name}@{version}"] = ids

    OUT.write_text(json.dumps(snapshot, indent=2, sort_keys=True) + "\n")
    print(f"\n{len(snapshot)} affected of {len(pkgs)} queried -> {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
