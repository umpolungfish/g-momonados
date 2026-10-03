#!/usr/bin/env python3
"""Independent semiprime controls for the complete recycled QPE executor."""
import argparse
import json
import math
import os
from pathlib import Path
import random
import re
import signal
import subprocess
import time


def is_prime(value):
    if value < 2:
        return False
    return all(value % divisor for divisor in range(2, math.isqrt(value) + 1))


def draw_prime(rng, bits):
    while True:
        value = rng.randrange(1 << (bits - 1), 1 << bits) | 1
        if is_prime(value):
            return value


def main():
    raise SystemExit("Retired: this launcher uses small factoring controls. Factoring tests require semiprimes of at least 128 bits; use the native runner.")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default="target/debug/g-momonados")
    parser.add_argument("--root", required=True)
    parser.add_argument("--timeout", type=int, default=180)
    parser.add_argument("--shots", type=int, default=12)
    parser.add_argument("--factor-widths", nargs="+", default=["6x6", "8x8", "8x8", "6x10", "10x10"])
    parser.add_argument("--cases", type=Path, help="replay every case from a previous independent draw")
    args = parser.parse_args()
    root = Path(args.root).resolve()
    root.mkdir(parents=True, exist_ok=False)
    rng = random.SystemRandom()
    cases = json.loads(args.cases.read_text()) if args.cases else []
    if not args.cases:
        for widths in args.factor_widths:
            p_bits, q_bits = map(int, widths.split("x"))
            if not (3 <= p_bits <= 16 and 3 <= q_bits <= 16):
                parser.error("trial-division prime controls require factor widths in 3..16")
            p, q = draw_prime(rng, p_bits), draw_prime(rng, q_bits)
            while p == q:
                q = draw_prime(rng, q_bits)
            cases.append({"n": p * q, "expected_primes": sorted([p, q]),
                          "factor_widths": widths, "initial_base": 2})
    (root / "cases.json").write_text(json.dumps(cases, indent=2) + "\n")
    binary = Path(args.binary).resolve()
    results = []
    for index, case in enumerate(cases):
        case_root = root / f"case_{index}"
        resource_log = root / f"case_{index}.resources.txt"
        command = ["/usr/bin/time", "-v", "-o", str(resource_log), str(binary),
                   "phase_unbraid", "gpu", str(case["n"]), "2", "4096",
                   str(case_root), str(args.shots)]
        start = time.monotonic()
        process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                   text=True, start_new_session=True)
        try:
            stdout, stderr = process.communicate(timeout=args.timeout)
            output = stdout + stderr
            pairs = re.findall(r"^factors = (\d+) × (\d+)$", output, re.MULTILINE)
            pair = sorted(map(int, pairs[-1])) if pairs else None
            verified = (process.returncode == 0 and pair == case["expected_primes"]
                        and math.prod(pair) == case["n"]
                        and "verified word product = true" in output)
            result = {**case, "verified": verified, "returncode": process.returncode,
                      "peak_residue_support": max(map(int, re.findall(r"peak residue support=(\d+)", output)), default=0),
                      "phase_bins": re.findall(r"k=(\d+)", output)}
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGTERM)
            stdout, stderr = process.communicate()
            output = stdout + stderr
            result = {**case, "verified": False, "timeout_seconds": args.timeout}
        result["elapsed_seconds"] = time.monotonic() - start
        (root / f"case_{index}.output.txt").write_text(output)
        results.append(result)
        (root / "results.json").write_text(json.dumps(results, indent=2) + "\n")
        print(json.dumps(result), flush=True)
    return 0 if all(result["verified"] for result in results) else 1


if __name__ == "__main__":
    raise SystemExit(main())
