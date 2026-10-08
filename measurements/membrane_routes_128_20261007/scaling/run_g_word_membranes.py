import json
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent
BIN = ROOT.parents[2] / "membranes" / "target" / "release"
SIZES = (256, 512, 1048)
CAP_SECONDS = 30
FIXED = (
    "membrane_aggregate_phase", "membrane_closed_divisor_search",
    "membrane_imscribing_factorizer", "membrane_factor_separating_crossing",
    "membrane_nested_instant_tower", "membrane_instant_nested_phase",
    "membrane_fully_nested_phase", "membrane_phase_based",
    "membrane_frobenius_circuit", "membrane_trilattice_native",
    "membrane_carry_fuse_closure", "membrane_prime_inverse_braider",
    "membrane_full_enfolding_nesting",
)
OTHER = ()
results = []

for bits in SIZES:
    source = (ROOT / str(bits) / "source.imasm").read_text().strip()
    if not source.startswith("⊢") or any(c.isascii() and c.isdigit() for c in source):
        raise SystemExit(f"{bits}: input is not a canonical IMASM word")
    for name in (*FIXED, *OTHER):
        args = [str(BIN / name), source]
        args += ["steps", "25000"]
        started = time.monotonic()
        try:
            run = subprocess.run(args, capture_output=True, text=True,
                                 timeout=CAP_SECONDS, check=False)
            elapsed = time.monotonic() - started
            status = "exit"
            output = run.stdout + run.stderr
            code = run.returncode
        except subprocess.TimeoutExpired as error:
            elapsed = time.monotonic() - started
            status = "timeout"
            stdout = error.stdout or b""
            stderr = error.stderr or b""
            output = (stdout.decode(errors="replace") if isinstance(stdout, bytes) else stdout)
            output += (stderr.decode(errors="replace") if isinstance(stderr, bytes) else stderr)
            code = None
        filename = f"{bits}/{name}.log"
        (ROOT / filename).write_text(output)
        row = {"bits": bits, "name": name, "input_representation": "canonical IMASM numeral word",
               "elapsed_seconds": round(elapsed, 6), "timeout_seconds": CAP_SECONDS,
               "status": status, "returncode": code,
               "factor_returned": "product_closes=true" in output or "verified=true" in output,
               "output_file": filename}
        results.append(row)
        (ROOT / "g_word_membranes_sweep.json").write_text(json.dumps(results, indent=2) + "\n")
        print(json.dumps(row), flush=True)
