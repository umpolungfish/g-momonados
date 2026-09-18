#!/usr/bin/env python3
"""Turn an OS ABC stream artifact into an exact Lean certificate module."""
import argparse
import json
import subprocess
from fractions import Fraction
from pathlib import Path

def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("stream_json", type=Path)
    ap.add_argument("--output", type=Path)
    ap.add_argument("--check", action="store_true", help="run lake env lean after generation")
    ap.add_argument("--vox", type=Path, metavar="SO", help="audit a compiled shared object with the OS Vox binary")
    args = ap.parse_args()
    line = next((s for s in args.stream_json.read_text().splitlines()
                 if s.lstrip().startswith("{")), None)
    if line is None:
        ap.error("no JSON object found")
    doc = json.loads(line)
    rows = doc["measurements"]
    if not rows:
        ap.error("measurements is empty")
    cutoffs = [int(r["cutoff"]) for r in rows]
    if cutoffs != sorted(set(cutoffs)):
        ap.error("cutoffs must be strictly increasing")
    eps = Fraction(str(doc["epsilon"])).limit_denominator(1000000)
    root = Path(__file__).resolve().parents[2] / "p4rakernel" / "p4ramill"
    generator = root / "scripts" / "abc_emit_window_certificate.py"
    output = args.output or root / "Imscribing" / f"ABC_Window{cutoffs[-1]}_E{eps.numerator}_{eps.denominator}.lean"
    cmd = ["python3", str(generator), "--cutoff", str(cutoffs[-1]),
           "--epsilon", f"{eps.numerator}/{eps.denominator}", "--output", str(output)]
    subprocess.run(cmd, check=True)
    if args.check:
        subprocess.run(["lake", "env", "lean", str(output)], cwd=root, check=True)
    if args.vox:
        vox = Path(__file__).resolve().parents[1] / "Vox" / "target" / "release" / "vox"
        if not vox.exists():
            vox = Path("/home/mrnob0dy666/imsgct/Vox/target/release/vox")
        subprocess.run([str(vox), str(args.vox)], check=True)
    print(json.dumps({"output": str(output), "cutoff": cutoffs[-1],
                      "epsilon": f"{eps.numerator}/{eps.denominator}",
                      "checked": args.check, "vox": str(args.vox) if args.vox else None}, indent=2))
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
