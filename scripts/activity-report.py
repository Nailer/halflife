#!/usr/bin/env python3
"""Evidence of activity on Halflife, from sources we do not control.

Everything here is read from GitHub and Solana devnet at run time, so a judge
can re-run it and get the same answer. Nothing is stored about individual
visitors: GitHub reports aggregate counts, and the chain is public.

    ./scripts/activity-report.py                  # print to stdout
    ./scripts/activity-report.py --out docs/submission/activity.md

Needs `gh` logged in as a repo admin for traffic numbers (GitHub only reports
those to the owner, and only for the last 14 days).
"""
import argparse, json, subprocess, sys
from datetime import datetime, timezone

REPO = "Nailer/halflife"
RPC = "https://api.devnet.solana.com"
PROGRAMS = {
    "Passport registry": "CkDhRfJRiGEa3kgnEUEvCBgyht62MTkDD6e754DLtB2",
    "Consumer": "BAcrrJYj5Y5DfcqnHgDwm5rhJUJdowZh25NvFLJAUzSW",
}
SITE = "https://halflife-control-room.vercel.app"


def gh(path):
    try:
        out = subprocess.run(["gh", "api", path], capture_output=True, text=True, check=True).stdout
        return json.loads(out)
    except Exception as e:  # missing gh, no admin rights, offline
        return {"_error": str(e).splitlines()[0][:120]}


def rpc(method, params):
    # curl rather than urllib: python.org builds on macOS ship without a CA bundle.
    body = json.dumps({"jsonrpc": "2.0", "id": 1, "method": method, "params": params})
    out = subprocess.run(["curl", "-sS", "--max-time", "30", RPC, "-H", "content-type: application/json",
                          "-d", body], capture_output=True, text=True, check=True).stdout
    return json.loads(out).get("result")


def ts(t):
    return datetime.fromtimestamp(t, timezone.utc).strftime("%Y-%m-%d %H:%M UTC")


def chain_rows():
    rows = []
    for name, pid in PROGRAMS.items():
        sigs, before = [], None
        while True:  # page through everything, 1000 at a time
            opts = {"limit": 1000, **({"before": before} if before else {})}
            page = rpc("getSignaturesForAddress", [pid, opts]) or []
            sigs += page
            if len(page) < 1000:
                break
            before = page[-1]["signature"]
        if not sigs:
            rows.append((name, pid, 0, 0, "-", "-"))
            continue
        failed = sum(1 for s in sigs if s.get("err"))
        times = [s["blockTime"] for s in sigs if s.get("blockTime")]
        rows.append((name, pid, len(sigs), failed, ts(min(times)), ts(max(times))))
    return rows


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out")
    out = ap.parse_args().out

    L = [f"# Halflife activity report", "",
         f"Generated {ts(datetime.now(timezone.utc).timestamp())} by `scripts/activity-report.py`. "
         "Every number is read live from GitHub or Solana devnet; re-run the script to check it.", ""]

    # --- on-chain ---
    L += ["## On-chain (Solana devnet)", "",
          "| Program | Address | Transactions | Failed | First | Latest |", "|---|---|---|---|---|---|"]
    try:
        for n, p, c, f, a, b in chain_rows():
            L.append(f"| {n} | [`{p[:6]}…{p[-4:]}`](https://explorer.solana.com/address/{p}?cluster=devnet) | {c} | {f} | {a} | {b} |")
    except Exception as e:
        L.append(f"| _devnet RPC unreachable: {e}_ | | | | | |")
    L.append("")

    # --- repository ---
    repo = gh(f"repos/{REPO}")
    L += ["## Repository", ""]
    if "_error" in repo:
        L.append(f"_unavailable: {repo['_error']}_")
    else:
        commits = gh(f"repos/{REPO}/commits?per_page=1")
        L += [f"- Stars {repo['stargazers_count']} · forks {repo['forks_count']} · open issues {repo['open_issues_count']}",
              f"- Created {repo['created_at'][:10]} · last push {repo['pushed_at'][:10]}"]
        runs = gh(f"repos/{REPO}/actions/runs?per_page=100")
        if "_error" not in runs:
            rs = runs["workflow_runs"]
            ok = sum(1 for r in rs if r["conclusion"] == "success")
            L.append(f"- CI: {len(rs)} most recent runs, {ok} passed")
    L.append("")

    # --- traffic (owner only, 14 days) ---
    L += ["## Traffic to the repository (last 14 days, aggregate only)", ""]
    for label, ep in (("Views", "views"), ("Clones", "clones")):
        d = gh(f"repos/{REPO}/traffic/{ep}")
        L.append(f"- {label}: " + (f"{d['count']} total, {d['uniques']} unique" if "_error" not in d else f"_unavailable ({d['_error']})_"))
    ref = gh(f"repos/{REPO}/traffic/popular/referrers")
    if isinstance(ref, list) and ref:
        L.append("- Referrers: " + ", ".join(f"{r['referrer']} ({r['uniques']})" for r in ref[:5]))
    L += ["", "## Live site", "",
          f"- {SITE}",
          "- Visitor counts for the site come from Vercel Web Analytics (cookieless, no individual "
          "records). Open the project's Analytics tab to show them.", ""]

    text = "\n".join(L)
    if out:
        open(out, "w").write(text + "\n")
        print(f"wrote {out}")
    else:
        print(text)


if __name__ == "__main__":
    sys.exit(main())
