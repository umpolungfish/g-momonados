#!/usr/bin/env python3
"""Run the four word-input order membranes serially on balanced semiprimes."""

import json
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent
BIN = Path(__file__).resolve().parents[3] / "membranes" / "target" / "release"
SIZES = (256, 512, 1048)
CAP = 2048
TIMEOUT = 90
ROUTES = (
    "membrane_instant_read",
    "membrane_squaring_cycle",
    "membrane_shor_order",
    "membrane_quantum_phase",
)
results = []

for bits in SIZES:
    source = (ROOT / str(bits) / "source.imasm").read_text().strip()
    for route in ROUTES:
        output_path = ROOT / str(bits) / f"{route}_word.log"
        started = time.perf_counter()
        try:
            run = subprocess.run(
                [str(BIN / route), source, str(CAP)], capture_output=True,
                text=True, timeout=TIMEOUT, check=False,
            )
            status, returncode = "exit", run.returncode
            output = run.stdout + run.stderr
        except subprocess.TimeoutExpired as error:
            status, returncode = "timeout", None
            stdout = error.stdout.decode(errors="replace") if isinstance(error.stdout, bytes) else error.stdout or ""
            stderr = error.stderr.decode(errors="replace") if isinstance(error.stderr, bytes) else error.stderr or ""
            output = stdout + stderr
        elapsed = time.perf_counter() - started
        output_path.write_text(output)
        results.append({
            "bits": bits,
            "route": route,
            "input_representation": "canonical IMASM numeral word",
            "iteration_cap": CAP,
            "timeout_seconds": TIMEOUT,
            "elapsed_seconds": round(elapsed, 6),
            "status": status,
            "returncode": returncode,
            "factor_returned": " = " in output and " x " in output,
            "output_file": output_path.name,
        })
        (ROOT / "order_membranes_sweep.json").write_text(json.dumps(results, indent=2) + "\n")
        print(json.dumps(results[-1]), flush=True)
