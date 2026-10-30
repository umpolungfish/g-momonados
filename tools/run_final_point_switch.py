"""Compile an N-only decimal source to its evaluated switch, then verify both returns.

Numerical projection runs in Cargo. Native and IMASM execution receive no input.
No time, instruction, or memory limit is imposed by this driver.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import time


def digest(path):
    h = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def verify(path, n):
    with path.open() as stream:
        returned = json.load(stream)
    if (returned.get("schema") != "cargo-evaluated-point-switch/v1"
            or returned.get("input_boundary") != "N-only"
            or returned.get("N") != str(n)
            or returned.get("product_verified") is not True
            or returned.get("projection_complete") is not True
            or returned.get("source_inputs_at_trigger") != 0
            or returned.get("numerical_operations_at_trigger") != 0):
        raise RuntimeError("returned image does not certify the selected preparation boundary")
    d, q = map(int, returned["factors"])
    if not (1 < d < n and 1 < q < n and n % d == 0 and d * q == n):
        raise RuntimeError("emitted factor marks fail exact source-product closure")
    if not returned.get("retained_membrane"):
        raise RuntimeError("returned image lost its retained membrane")
    return {"divisor": str(d), "quotient": str(q), "zero_remainder": True,
            "exact_product": True, "preparation_seconds": returned["preparation_seconds"]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path, help="file containing N alone in decimal")
    parser.add_argument("receipt_dir", type=Path, help="new directory; prior receipts are preserved")
    args = parser.parse_args()
    project = Path(__file__).resolve().parents[1]
    source = args.source.resolve()
    raw = source.read_text().strip()
    if not raw or not raw.isascii() or not raw.isdecimal() or int(raw) <= 2:
        parser.error("source must contain one decimal N greater than two")
    n = int(raw)
    receipts = args.receipt_dir.resolve()
    receipts.mkdir(parents=True, exist_ok=False)
    source_hash = digest(source)
    shutil.copy2(source, receipts / "source.txt")
    # Use the receipt's immutable input copy for the actual Cargo build.
    target = project / "target/projector-static-20261021"
    env = os.environ.copy()
    for key in ["FINAL_POINT_SOURCE_FILE", "POINT_CARRIER_SOURCE_FILE",
                "RESIDENT_SOURCE_FILE", "RESIDENT_PREPARED_FILE", "QUQUART_PREPARED_FILE"]:
        env.pop(key, None)
    env.update(POINT_SOURCE_FILE=str(receipts / "source.txt"),
               CARGO_TARGET_DIR=str(target),
               RUSTFLAGS="-D warnings -C target-feature=+crt-static")
    launch = {"input_boundary": "N-only", "N": str(n), "source_sha256": source_hash,
              "source_inputs_at_trigger": 0, "instruction_limit": None,
              "started_unix": time.time()}
    (receipts / "launch.json").write_text(json.dumps(launch, indent=2) + "\n")

    def run(argv, name, run_env=env):
        started = time.monotonic()
        with (receipts / f"{name}.stdout").open("wb") as out, (receipts / f"{name}.stderr").open("wb") as err:
            subprocess.run(list(map(str, argv)), cwd=project, env=run_env,
                           stdout=out, stderr=err, check=True)
        return time.monotonic() - started

    compilation = run(["cargo", "build", "--offline", "--release", "--target",
                       "x86_64-unknown-linux-gnu", "--features", "hosted",
                       "--example", "point_switch", "-j", "1", "-vv"], "cargo")
    binary = receipts / "point_switch"
    shutil.copy2(target / "x86_64-unknown-linux-gnu/release/examples/point_switch", binary)
    trigger_env = env.copy()
    trigger_env.pop("POINT_SOURCE_FILE", None)
    native_seconds = run([binary], "native", trigger_env)
    checked = verify(receipts / "native.stdout", n)
    symbols = subprocess.check_output(["nm", "--demangle", str(binary)], text=True)
    (receipts / "symbols.txt").write_text(symbols)
    pending = [line for line in symbols.splitlines() if any(name in line for name in
               ["idempotent_projector::Projector::", "conventional_spectrum::PointMembrane::",
                "ququart_decision::DecisionArena::"])]
    if pending:
        raise RuntimeError("final executable retains numerical preparation symbols")
    vox = project.parent / "Vox/target/release/vox"
    module = receipts / "switch.imasm"
    export_seconds = run([vox, "imasm", binary, "--output", module], "export")
    vm_env = trigger_env.copy()
    vm_env.update(VOX_STEPS="0", VOX_PROGRESS="1")
    vm_seconds = run([vox, "run", module], "vox", vm_env)
    if verify(receipts / "vox.stdout", n) != checked:
        raise RuntimeError("native and IMASM factor boundaries differ")
    if digest(receipts / "source.txt") != source_hash:
        raise RuntimeError("source copy changed during preparation")
    receipt = {**launch, **checked, "compilation_seconds": compilation,
               "native_process_seconds": native_seconds, "imasm_export_seconds": export_seconds,
               "imasm_process_seconds": vm_seconds, "binary_sha256": digest(binary),
               "module_sha256": digest(module), "preparation_symbols_retained": pending,
               "native_and_imasm_verified": True}
    (receipts / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(receipt))


if __name__ == "__main__":
    main()
