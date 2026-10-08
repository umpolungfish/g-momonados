#!/usr/bin/env python3
"""Run word-native ECM on 256/512/1048-bit semiprimes with a 31-bit factor."""

import json
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent / "ecm_witness_p31"
BIN = Path(__file__).resolve().parents[3] / "membranes" / "target" / "release" / "membrane_ecm_extract"
results = []

for bits in (256, 512, 1048):
    source = (ROOT / str(bits) / "source.imasm").read_text().strip()
    started = time.perf_counter()
    run = subprocess.run(
        [str(BIN), source, "5000", "50000", "100"],
        capture_output=True,
        text=True,
        timeout=120,
        check=False,
    )
    elapsed = time.perf_counter() - started
    output = run.stdout + run.stderr
    output_path = ROOT / str(bits) / "membrane_ecm_extract.log"
    output_path.write_text(output)
    result = {
        "bits": bits,
        "route": "membrane_ecm_extract",
        "factor_bits": 31,
        "B1": 5000,
        "B2": 50000,
        "curves": 100,
        "input_representation": "canonical IMASM numeral word",
        "elapsed_seconds": round(elapsed, 6),
        "returncode": run.returncode,
        "factor_returned": "product_closes=true" in output,
        "output_file": f"{bits}/membrane_ecm_extract.log",
    }
    results.append(result)
    print(json.dumps(result), flush=True)

(ROOT / "ecm_sweep.json").write_text(json.dumps(results, indent=2) + "\n")
