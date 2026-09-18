#!/usr/bin/env python3
"""Validate an ABC stream JSON artifact and emit Lean-generator input."""
import json
import sys

def main() -> int:
    if len(sys.argv) == 2 and sys.argv[1] in ("-h", "--help"):
        print("usage: abc_json_to_certificate_manifest.py STREAM.json")
        print("validate a G-mOMonadOS abc stream JSON artifact and emit Lean-generator input")
        return 0
    if len(sys.argv) != 2:
        print(f"usage: {sys.argv[0]} STREAM.json", file=sys.stderr)
        return 2
    lines = open(sys.argv[1], encoding="utf-8").read().splitlines()
    line = next((s for s in lines if s.lstrip().startswith('{')), None)
    if line is None:
        raise SystemExit("no JSON object found")
    doc = json.loads(line)
    eps = float(doc["epsilon"])
    rows = doc["measurements"]
    if not rows:
        raise SystemExit("measurements is empty")
    cutoffs = [int(r["cutoff"]) for r in rows]
    if cutoffs != sorted(set(cutoffs)):
        raise SystemExit("cutoffs must be strictly increasing")
    for r in rows:
        t = r["triple"]
        if int(t["a"]) + int(t["b"]) != int(t["c"]):
            raise SystemExit(f"invalid sum at cutoff {r['cutoff']}")
        if not all(r[k] for k in ("radical_calibrated", "height_calibrated", "cofinal")):
            raise SystemExit(f"calibration/cofinal flag is false at cutoff {r['cutoff']}")
    print(f"epsilon={eps:.12g}")
    print("# cutoff a b c discrepancy")
    for r in rows:
        t = r["triple"]
        print(f"{int(r['cutoff'])} {int(t['a'])} {int(t['b'])} {int(t['c'])} {float(r['discrepancy']):.12f}")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
