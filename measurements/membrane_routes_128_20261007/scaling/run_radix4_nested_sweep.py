#!/usr/bin/env python3
"""Run the nested radix-four word on scaled and balanced IMASM numerals."""

import json
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent
BASE = Path(__file__).resolve().parents[3]
BIN = BASE / "membranes" / "target" / "release" / "membrane_radix4_factor"
CASES = [
    (bits, ROOT / "ecm_witness" / str(bits) / "source.imasm", "small_factor")
    for bits in (256, 512, 1048)
]
CASES.append((256, ROOT / "256" / "source.imasm", "balanced_control"))
records = []

for bits, source_path, label in CASES:
    numeral = source_path.read_text().strip()
    started = time.perf_counter()
    try:
        run = subprocess.run(
            [str(BIN), numeral, "2000000"],
            capture_output=True,
            text=True,
            timeout=180,
            check=False,
        )
        elapsed = time.perf_counter() - started
        output = run.stdout + run.stderr
        returncode = run.returncode
        timed_out = False
    except subprocess.TimeoutExpired as exc:
        elapsed = time.perf_counter() - started
        stdout = exc.stdout or ""
        stderr = exc.stderr or ""
        if isinstance(stdout, bytes): stdout = stdout.decode(errors="replace")
        if isinstance(stderr, bytes): stderr = stderr.decode(errors="replace")
        output = stdout + stderr
        returncode = None
        timed_out = True
    log = ROOT / "radix4_nested_runs" / label / str(bits) / "membrane_radix4_nested.log"
    log.parent.mkdir(parents=True, exist_ok=True)
    log.write_text(output)
    factor_line = next((line for line in output.splitlines() if line.startswith("factor=")), "")
    cofactor_line = next((line for line in output.splitlines() if line.startswith("cofactor=")), "")
    record = {
        "bits": bits,
        "case": label,
        "input_representation": "canonical IMASM numeral word",
        "elapsed_seconds": round(elapsed, 6),
        "returncode": returncode,
        "timed_out": timed_out,
        "factor": factor_line.removeprefix("factor=") or None,
        "cofactor": cofactor_line.removeprefix("cofactor=") or None,
        "product_closes": "product_closes=true" in output,
        "output": str(log.relative_to(ROOT)),
    }
    records.append(record)
    print(json.dumps({
        "bits": bits,
        "case": label,
        "elapsed_seconds": record["elapsed_seconds"],
        "product_closes": record["product_closes"],
        "timed_out": timed_out,
    }), flush=True)

out = ROOT / "radix4_nested_sweep.json"
out.write_text(json.dumps(records, indent=2) + "\n")
