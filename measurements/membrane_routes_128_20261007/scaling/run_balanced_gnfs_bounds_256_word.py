#!/usr/bin/env python3
"""Tune the GNFS factor-base bound on the balanced 256-bit IMASM control."""

import json
import os
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent
SOURCE = (ROOT / "256" / "source.imasm").read_text().strip()
BIN = Path(__file__).resolve().parents[3] / "target" / "release" / "g-momonados"
environment = os.environ.copy()
environment["GNFS_GPU"] = "1"
results = []

for bound in (2000, 5000, 10000):
    started = time.perf_counter()
    timed_out = False
    try:
        run = subprocess.run(
            [str(BIN), "gpu_gnfs", SOURCE, str(bound)],
            cwd=Path(__file__).resolve().parents[3],
            env=environment,
            capture_output=True,
            text=True,
            timeout=120,
            check=False,
        )
        output = run.stdout + run.stderr
        returncode = run.returncode
    except subprocess.TimeoutExpired as error:
        output = error.stdout or ""
        if isinstance(output, bytes):
            output = output.decode(errors="replace")
        stderr = error.stderr or ""
        if isinstance(stderr, bytes):
            stderr = stderr.decode(errors="replace")
        output += stderr
        returncode = None
        timed_out = True
    elapsed = time.perf_counter() - started
    log_name = f"gpu_gnfs_balanced_256_B{bound}.log"
    (ROOT / log_name).write_text(output)
    result = {
        "bits": 256,
        "route": "gpu_gnfs",
        "bound": bound,
        "input_representation": "canonical IMASM numeral word",
        "elapsed_seconds": round(elapsed, 6),
        "timed_out": timed_out,
        "returncode": returncode,
        "factor_returned": "product_closes=true" in output,
        "output_file": log_name,
    }
    results.append(result)
    print(json.dumps(result), flush=True)

(ROOT / "gpu_gnfs_balanced_256_bounds.json").write_text(json.dumps(results, indent=2) + "\n")
