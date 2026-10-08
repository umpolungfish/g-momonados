#!/usr/bin/env python3
"""Run the nested-word ECM route on canonical 256/512/1048-bit witnesses."""

import json
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent
BIN = Path(__file__).resolve().parents[3] / "membranes" / "target" / "release"
records = []

for bits in (256, 512, 1048):
    case = ROOT / "ecm_witness_p31" / str(bits)
    numeral = (case / "source.imasm").read_text().strip()
    started = time.perf_counter()
    run = subprocess.run(
        [str(BIN / "membrane_ecm_extract"), numeral, "5000", "50000", "100"],
        capture_output=True,
        text=True,
        timeout=120,
        check=False,
    )
    elapsed = time.perf_counter() - started
    output = run.stdout + run.stderr
    log = case / "membrane_ecm_nested_word.log"
    log.write_text(output)
    factor_line = next((line for line in output.splitlines() if line.startswith("factor=")), "")
    cofactor_line = next((line for line in output.splitlines() if line.startswith("cofactor=")), "")
    records.append({
        "bits": bits,
        "input_representation": "canonical IMASM numeral word",
        "elapsed_seconds": round(elapsed, 6),
        "returncode": run.returncode,
        "factor": factor_line.removeprefix("factor=") or None,
        "cofactor": cofactor_line.removeprefix("cofactor=") or None,
        "product_closes": "product_closes=true" in output,
        "output": str(log.relative_to(ROOT)),
    })
    print(json.dumps({
        "bits": bits,
        "elapsed_seconds": records[-1]["elapsed_seconds"],
        "product_closes": records[-1]["product_closes"],
    }), flush=True)

out = ROOT / "ecm_nested_word_sweep.json"
out.write_text(json.dumps(records, indent=2) + "\n")
