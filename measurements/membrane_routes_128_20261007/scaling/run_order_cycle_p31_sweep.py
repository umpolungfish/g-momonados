#!/usr/bin/env python3
"""Measure the shared WordTape order-cycle route at all target widths."""

import json
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent / "ecm_witness_p31"
BIN = Path(__file__).resolve().parents[3] / "membranes" / "target" / "release" / "membrane_shor_order"
results = []

for bits in (256, 512, 1048):
    source = (ROOT / str(bits) / "source.imasm").read_text().strip()
    started = time.perf_counter()
    run = subprocess.run(
        [str(BIN), source, "2048"],
        capture_output=True,
        text=True,
        timeout=90,
        check=False,
    )
    elapsed = time.perf_counter() - started
    output = run.stdout + run.stderr
    output_path = ROOT / str(bits) / "membrane_shor_order.log"
    output_path.write_text(output)
    result = {
        "bits": bits,
        "route": "membrane_shor_order",
        "input_representation": "canonical IMASM numeral word",
        "iteration_cap": 2048,
        "elapsed_seconds": round(elapsed, 6),
        "returncode": run.returncode,
        "factor_returned": "product_closes=true" in output,
        "output_file": f"{bits}/membrane_shor_order.log",
    }
    results.append(result)
    print(json.dumps(result), flush=True)

(ROOT / "order_cycle_sweep.json").write_text(json.dumps(results, indent=2) + "\n")
