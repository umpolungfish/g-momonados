"""Bound the complete-amplitude frame instrument to retained N-only sources."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import time

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[1]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    args = argparse.ArgumentParser()
    args.add_argument("case", choices=["control_208", "locked_512"])
    args.add_argument("--seconds", type=int, default=300)
    selected = args.parse_args()
    source = HERE / "inputs" / {
        "control_208": "htsx_balanced_control_208.imasm",
        "locked_512": "fresh_balanced_512.imasm",
    }[selected.case]
    operator = HERE / "htsx_split_fuse_operator.json"
    executable = HERE / "banked_circuit.probe"
    if not executable.exists():
        shutil.copy2(REPO / "target/release/resident_frobenius_exact_probe", executable)
    # Reuse this retained image for the paired campaign, even if target/ rebuilds.
    before = digest(executable)
    implementation = ["src/resident_frame.rs", "src/resident_glut.rs", "src/ququart_decision.rs",
                      "src/bin/resident_frobenius_exact_probe.rs", "vendor/imasm_core/src/flow.rs"]
    bindings = {"executable_sha256": before, "source_sha256": digest(source),
                "operator_sha256": digest(operator), "input_boundary": "N-only IMASM source",
                "implementation_sha256": {path: digest(REPO / path) for path in implementation},
                "run_bound_seconds": selected.seconds}
    started = time.monotonic()
    process = subprocess.run(["timeout", str(selected.seconds), str(executable), str(source), str(operator)],
                             capture_output=True, text=True)
    elapsed = time.monotonic() - started
    stem = HERE / f"banked_circuit_{selected.case}.bound"
    stem.with_suffix(".bound.stdout").write_text(process.stdout)
    stem.with_suffix(".bound.stderr").write_text(process.stderr)
    report = json.loads(process.stdout) if process.returncode == 0 else None
    if report is not None:
        for frame_name in ["forward_frame", "reverse_frame"]:
            frame = report[frame_name]
            for key in ["exact_split_return", "live_register_cleared", "exact_banked_return",
                        "exact_transformed_return", "exact_dual_return_after_determinant_normalization",
                        "information_arm_loss_detected"]:
                if frame[key] is not True:
                    raise ValueError(f"{frame_name}.{key} did not pass")
        if report["exact_complex_gram_coordinates"] != 16 or report["maximum_absolute_gram_delta"] != "0":
            raise ValueError("complex coordinates did not return exactly")
        for key in ["exact_full_state_return_after_determinant_normalization",
                    "exact_prepared_state_restoration", "exact_mass_return_after_determinant_normalization"]:
            if report[key] is not True:
                raise ValueError(f"{key} did not pass")
    if digest(executable) != before:
        raise ValueError("retained executable changed during its run")
    receipt = {"case": selected.case, "bindings": bindings, "returncode": process.returncode,
               "process_seconds": elapsed, "report": report}
    stem.with_suffix(".bound.receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps({"case": selected.case, "returncode": process.returncode,
                      "process_seconds": elapsed, "complete_frame_return": report is not None}))
    if process.returncode:
        raise SystemExit(process.returncode)


if __name__ == "__main__":
    main()
