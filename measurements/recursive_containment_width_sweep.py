#!/usr/bin/env python3
"""Fresh-process width sweep for the recursive-containment phase relation."""

import argparse
import json
import statistics
import subprocess
import sys
import time
from pathlib import Path


WIDTHS = (
    128, 256, 512, 1024, 2048, 4096, 8192, 16384,
    32768, 65536, 131072, 262144, 524288, 1048576, 2097152, 4194304,
)

if hasattr(sys, "set_int_max_str_digits"):
    sys.set_int_max_str_digits(0)


def run(width: int, samples: int, executable: Path) -> dict:
    p = (1 << (width - 1)) + 11
    q = (1 << (width - 1)) + 57
    target = p * q
    witness = f"{target} = {p} x {q} (verified)"
    timings = []
    for sample in range(samples):
        started = time.perf_counter_ns()
        completed = subprocess.run(
            [str(executable), "--selector-relation", str(target), str(width)],
            check=False,
            capture_output=True,
            text=True,
        )
        elapsed_ms = (time.perf_counter_ns() - started) / 1_000_000
        if completed.returncode != 0 or witness not in completed.stdout:
            raise RuntimeError(
                f"width {width}, sample {sample}: exit {completed.returncode}\n"
                f"stdout:\n{completed.stdout}\nstderr:\n{completed.stderr}"
            )
        timings.append(elapsed_ms)
    return {
        "p": str(p),
        "q": str(q),
        "target": str(target),
        "factor_width": width,
        "target_bits": target.bit_length(),
        "samples": timings,
        "median_ms": statistics.median(timings),
        "minimum_ms": min(timings),
        "mean_ms": statistics.fmean(timings),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--samples", type=int, default=5)
    parser.add_argument("--widths", default=",".join(str(width) for width in WIDTHS),
                        help="comma-separated factor widths")
    parser.add_argument("--output", type=Path,
                        default=Path("measurements/recursive_containment_width_sweep.jsonl"))
    parser.add_argument("--executable", type=Path,
                        default=Path("target/release/g-momonados"))
    args = parser.parse_args()
    if args.samples < 1:
        raise SystemExit("--samples must be positive")
    widths = tuple(int(width) for width in args.widths.split(",") if width)
    with args.output.open("w") as results:
        for width in widths:
            row = run(width, args.samples, args.executable)
            results.write(json.dumps(row, sort_keys=True) + "\n")
            results.flush()
            print(
                f"w={width} target_bits={row['target_bits']} "
                f"median={row['median_ms']:.4f}ms min={row['minimum_ms']:.4f}ms "
                f"mean={row['mean_ms']:.4f}ms",
                flush=True,
            )


if __name__ == "__main__":
    main()
