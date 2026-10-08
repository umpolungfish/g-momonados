#!/usr/bin/env python3
"""Measure the shared fixed-word engine and ECM continuation on hard controls."""

import json
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent
BIN = Path(__file__).resolve().parents[3] / "membranes" / "target" / "release" / "membrane_aggregate_phase"
results = []

for bits in (256, 512, 1048):
    source = (ROOT / str(bits) / "source.imasm").read_text().strip()
    started = time.perf_counter()
    run = subprocess.run(
        [str(BIN), source, "steps", "25000"], capture_output=True,
        text=True, timeout=90, check=False,
    )
    elapsed = time.perf_counter() - started
    output = run.stdout + run.stderr
    name = "membrane_aggregate_phase_current.log"
    (ROOT / str(bits) / name).write_text(output)
    results.append({
        "bits": bits,
        "route": "membrane_aggregate_phase",
        "input_representation": "canonical IMASM numeral word",
        "frontier_steps": 25000,
        "elapsed_seconds": round(elapsed, 6),
        "returncode": run.returncode,
        "factor_returned": "product_closes=true" in output,
        "output_file": name,
    })
    print(json.dumps(results[-1]), flush=True)

(ROOT / "fixed_word_balanced_current.json").write_text(json.dumps(results, indent=2) + "\n")
