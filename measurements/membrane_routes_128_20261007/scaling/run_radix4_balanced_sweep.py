#!/usr/bin/env python3
"""Measure the word-native radix-four lift on the balanced scaling inputs."""

import json
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent
BIN = Path(__file__).resolve().parents[3] / "membranes" / "target" / "release" / "membrane_radix4_factor"
CAP = 200_000
TIMEOUT = 180
results = []

for bits in (256, 512, 1048):
    source = (ROOT / str(bits) / "source.imasm").read_text().strip()
    output_path = ROOT / str(bits) / "membrane_radix4_balanced.log"
    started = time.perf_counter()
    try:
        run = subprocess.run(
            [str(BIN), source, str(CAP)], capture_output=True, text=True,
            timeout=TIMEOUT, check=False,
        )
        status = "exit"
        stdout, stderr = run.stdout, run.stderr
        returncode = run.returncode
    except subprocess.TimeoutExpired as error:
        status = "timeout"
        stdout = error.stdout.decode() if isinstance(error.stdout, bytes) else error.stdout or ""
        stderr = error.stderr.decode() if isinstance(error.stderr, bytes) else error.stderr or ""
        returncode = None
    elapsed = time.perf_counter() - started
    output_path.write_text(stdout + stderr)
    results.append({
        "bits": bits,
        "input_representation": "canonical IMASM numeral word",
        "node_cap": CAP,
        "timeout_seconds": TIMEOUT,
        "elapsed_seconds": round(elapsed, 6),
        "status": status,
        "returncode": returncode,
        "factor_returned": "product_closes=true" in stdout,
        "output_file": output_path.name,
    })

(ROOT / "radix4_balanced_sweep.json").write_text(json.dumps(results, indent=2) + "\n")
print(json.dumps(results, indent=2))
