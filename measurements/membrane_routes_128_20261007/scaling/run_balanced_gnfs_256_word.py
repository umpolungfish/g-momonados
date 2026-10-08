#!/usr/bin/env python3
"""Run the GPU GNFS path on the balanced 256-bit IMASM control."""

import json
import os
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent
SOURCE = (ROOT / "256" / "source.imasm").read_text().strip()
OUTPUT = ROOT / "gpu_gnfs_balanced_256_word.log"
RESULT = ROOT / "gpu_gnfs_balanced_256_word.json"
environment = os.environ.copy()
environment["GNFS_GPU"] = "1"
started = time.perf_counter()

try:
    run = subprocess.run(
        ["target/release/g-momonados", "gpu_gnfs", SOURCE],
        cwd=Path(__file__).resolve().parents[3],
        env=environment,
        capture_output=True,
        text=True,
        timeout=300,
        check=False,
    )
    output = run.stdout + run.stderr
    returncode = run.returncode
    timed_out = False
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
OUTPUT.write_text(output)
result = {
    "bits": 256,
    "route": "gpu_gnfs",
    "input_representation": "canonical IMASM numeral word",
    "elapsed_seconds": round(elapsed, 6),
    "timed_out": timed_out,
    "returncode": returncode,
    "factor_returned": "product_closes=true" in output,
    "output_file": OUTPUT.name,
}
RESULT.write_text(json.dumps(result, indent=2) + "\n")
print(json.dumps(result))
