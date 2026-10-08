#!/usr/bin/env python3
"""Exercise the nested IMASM order frame on scaled IMASM semiprimes."""

import json
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent
BASE = Path(__file__).resolve().parents[3]
BIN = BASE / "membranes" / "target" / "release" / "membrane_shor_order"
CASES = [
    (bits, ROOT / "ecm_witness_p31" / str(bits) / "source.imasm", "small_factor")
    for bits in (256, 512, 1048)
]
CASES += [
    (bits, ROOT / str(bits) / "source.imasm", "balanced_control")
    for bits in (256, 512, 1048)
]
records = []

for bits, source_path, label in CASES:
    numeral = source_path.read_text().strip()
    started = time.perf_counter()
    run = subprocess.run(
        [str(BIN), numeral, "2048"],
        capture_output=True,
        text=True,
        timeout=90,
        check=False,
    )
    elapsed = time.perf_counter() - started
    output = run.stdout + run.stderr
    log = ROOT / "order_nested_runs" / label / str(bits) / "membrane_shor_order_nested.log"
    log.parent.mkdir(parents=True, exist_ok=True)
    log.write_text(output)
    record = {
        "bits": bits,
        "case": label,
        "input_representation": "canonical IMASM numeral word",
        "iteration_cap": 2048,
        "elapsed_seconds": round(elapsed, 6),
        "returncode": run.returncode,
        "collision": "no squaring-cycle collision" not in output,
        "factor_returned": "product_closes=true" in output,
        "output": str(log.relative_to(ROOT)),
    }
    records.append(record)
    print(json.dumps({
        "bits": bits,
        "case": label,
        "elapsed_seconds": record["elapsed_seconds"],
        "collision": record["collision"],
        "factor_returned": record["factor_returned"],
    }), flush=True)

out = ROOT / "order_nested_sweep.json"
out.write_text(json.dumps(records, indent=2) + "\n")
