#!/usr/bin/env python3
"""Run the shared fixed-word and ECM chain on the 31-bit-factor inputs."""

import json
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent / "ecm_witness_p31"
BIN = Path(__file__).resolve().parents[3] / "membranes" / "target" / "release" / "membrane_aggregate_phase"
results = []

for bits in (256, 512, 1048):
    source = (ROOT / str(bits) / "source.imasm").read_text().strip()
    started = time.perf_counter()
    try:
        run = subprocess.run(
            [str(BIN), source, "steps", "25000"],
            capture_output=True,
            text=True,
            timeout=120,
            check=False,
        )
        output = run.stdout + run.stderr
        returncode = run.returncode
        timed_out = False
    except subprocess.TimeoutExpired as error:
        output = (error.stdout or "") + (error.stderr or "")
        returncode = None
        timed_out = True
    elapsed = time.perf_counter() - started
    output_path = ROOT / str(bits) / "membrane_aggregate_phase.log"
    output_path.write_text(output)
    result = {
        "bits": bits,
        "route": "membrane_aggregate_phase",
        "factor_bits": 31,
        "frontier_steps": 25000,
        "input_representation": "canonical IMASM numeral word",
        "ecm_B1": 5000,
        "ecm_B2": 50000,
        "ecm_curves": 100,
        "elapsed_seconds": round(elapsed, 6),
        "timed_out": timed_out,
        "returncode": returncode,
        "factor_returned": "product_closes=true" in output,
        "output_file": f"{bits}/membrane_aggregate_phase.log",
    }
    results.append(result)
    print(json.dumps(result), flush=True)

(ROOT / "fixed_word_sweep.json").write_text(json.dumps(results, indent=2) + "\n")
