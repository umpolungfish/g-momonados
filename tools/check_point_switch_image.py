"""Compare the entire restored switch carrier byte-for-byte with Cargo's image."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("cargo_output", type=Path)
    parser.add_argument("receipt_dir", type=Path)
    args = parser.parse_args()
    args.receipt_dir.mkdir(parents=True, exist_ok=True)
    binary = args.receipt_dir / "point_switch"
    shutil.copy2(args.binary, binary)
    for name in ["point_preparation_receipt.json", "point_prepared.expected.json", "point_prepared.rs", "point_support.bin"]:
        shutil.copy2(args.cargo_output / name, args.receipt_dir / name)
    started = time.monotonic()
    with (args.receipt_dir / "restored.json").open("wb") as out, (args.receipt_dir / "restore.stderr").open("wb") as err:
        subprocess.run([str(binary.resolve()), "--dump-prepared"], stdout=out, stderr=err, check=True)
    seconds = time.monotonic() - started
    expected = (args.receipt_dir / "point_prepared.expected.json").read_bytes()
    restored = (args.receipt_dir / "restored.json").read_bytes().rstrip(b"\n")
    if expected != restored:
        raise RuntimeError("restored carrier differs from Cargo's prepared image")
    symbols = subprocess.check_output(["nm", str(binary)], text=True)
    preparers = [line for line in symbols.splitlines() if any(name in line for name in [
        "9Projector7compile", "13PointMembrane7prepare", "16PreparedProtocol7prepare", "4Plan4load", "9SelfModel7prepare"
    ])]
    preparation = json.loads((args.receipt_dir / "point_preparation_receipt.json").read_text())
    record = {
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "complete_prepared_image_sha256": hashlib.sha256(expected).hexdigest(),
        "complete_prepared_image_exact_return": True,
        "restore_and_full_dump_seconds": seconds,
        "preparation": preparation,
        "preparation_symbols_retained": preparers,
        "comparison_scope": "complete shared register, complex coefficients, support, all banks, tensor incidence, Y, SIC frame/dual, graphs, phase and self-model"
    }
    (args.receipt_dir / "receipt.json").write_text(json.dumps(record, indent=2))
    print(json.dumps({key: value for key, value in record.items() if key != "preparation"}))


if __name__ == "__main__":
    main()
