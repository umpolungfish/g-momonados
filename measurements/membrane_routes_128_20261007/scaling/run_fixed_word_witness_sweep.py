#!/usr/bin/env python3
"""Run the fixed-word membranes on the 256-, 512- and 1048-bit IMASM witnesses."""

import json
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent
BIN = Path(__file__).resolve().parents[3] / "membranes" / "target" / "release"
SIZES = (256, 512, 1048)
TIMEOUT = 60
FIXED = (
    "membrane_aggregate_phase", "membrane_closed_divisor_search",
    "membrane_imscribing_factorizer", "membrane_factor_separating_crossing",
    "membrane_nested_instant_tower", "membrane_instant_nested_phase",
    "membrane_fully_nested_phase", "membrane_phase_based",
    "membrane_frobenius_circuit", "membrane_trilattice_native",
    "membrane_carry_fuse_closure", "membrane_prime_inverse_braider",
    "membrane_full_enfolding_nesting",
)
results = []

for bits in SIZES:
    source = (ROOT / "ecm_witness" / str(bits) / "source.imasm").read_text().strip()
    for route in FIXED:
        started = time.perf_counter()
        try:
            run = subprocess.run(
                [str(BIN / route), source, "steps", "25000"],
                capture_output=True, text=True, timeout=TIMEOUT, check=False,
            )
            status, returncode = "exit", run.returncode
            output = run.stdout + run.stderr
        except subprocess.TimeoutExpired as error:
            status, returncode = "timeout", None
            stdout = error.stdout.decode(errors="replace") if isinstance(error.stdout, bytes) else error.stdout or ""
            stderr = error.stderr.decode(errors="replace") if isinstance(error.stderr, bytes) else error.stderr or ""
            output = stdout + stderr
        elapsed = time.perf_counter() - started
        output_path = ROOT / "ecm_witness" / str(bits) / f"{route}.log"
        output_path.write_text(output)
        results.append({
            "bits": bits,
            "route": route,
            "input_representation": "canonical IMASM numeral word",
            "frontier_steps": 25000,
            "timeout_seconds": TIMEOUT,
            "elapsed_seconds": round(elapsed, 6),
            "status": status,
            "returncode": returncode,
            "factor_returned": "product_closes=true" in output,
            "output_file": str(output_path.relative_to(ROOT)),
        })
        (ROOT / "fixed_word_witness_sweep.json").write_text(json.dumps(results, indent=2) + "\n")
        print(json.dumps(results[-1]), flush=True)
