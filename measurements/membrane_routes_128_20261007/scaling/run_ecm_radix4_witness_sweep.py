#!/usr/bin/env python3
"""Time word-native ECM and radix-four factoring on the three large witnesses."""

import json
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent
BIN = Path(__file__).resolve().parents[3] / "membranes" / "target" / "release"
results = []

for bits in (256, 512, 1048):
    source = (ROOT / "ecm_witness" / str(bits) / "source.imasm").read_text().strip()
    runs = (
        ("membrane_ecm_extract", ["500", "500", "1"]),
        ("membrane_radix4_factor", ["2000000"]),
    )
    for route, parameters in runs:
        started = time.perf_counter()
        run = subprocess.run(
            [str(BIN / route), source, *parameters], capture_output=True,
            text=True, timeout=120, check=False,
        )
        elapsed = time.perf_counter() - started
        output = run.stdout + run.stderr
        output_path = ROOT / "ecm_witness" / str(bits) / f"{route}.log"
        output_path.write_text(output)
        results.append({
            "bits": bits,
            "route": route,
            "input_representation": "canonical IMASM numeral word",
            "elapsed_seconds": round(elapsed, 6),
            "returncode": run.returncode,
            "factor_returned": "product_closes=true" in output,
            "output_file": str(output_path.relative_to(ROOT)),
        })
        print(json.dumps(results[-1]), flush=True)

(ROOT / "ecm_radix4_witness_sweep.json").write_text(json.dumps(results, indent=2) + "\n")
