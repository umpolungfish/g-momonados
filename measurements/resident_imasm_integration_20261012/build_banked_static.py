"""Bake the banked circuit and preserve its compilation/source bindings."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import time

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[1]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tag", default="banked_circuit_locked_512",
                        help="new receipt prefix; existing build records are preserved")
    parser.add_argument("--composed", action="store_true",
                        help="bake the complete modular permutation as the executing lane operation")
    parser.add_argument("--source", type=Path, default=HERE / "inputs/fresh_balanced_512.imasm",
                        help="N-only source word; defaults to the locked 512-bit source")
    args = parser.parse_args()
    if not re.fullmatch(r"[A-Za-z0-9_-]+", args.tag):
        parser.error("tag must contain only letters, digits, underscores, or hyphens")
    prefix = args.tag
    receipt_file = HERE / f"{prefix}.static-build.receipt.json"
    log = HERE / f"{prefix}.static-build.log"
    if log.exists() or receipt_file.exists():
        parser.error("choose a new tag to preserve the existing compilation record")
    source = args.source.resolve()
    operator = HERE / "htsx_split_fuse_operator.json"
    implementation = ["Cargo.toml", ".cargo/config.toml", "build.rs", "resident_build.rs", "src/lib.rs", "src/resident_frame.rs",
                      "src/resident_glut.rs", "src/ququart_decision.rs", "src/whole_register_phase.rs",
                      "src/reentry_powerset.rs", "src/baked_resident.rs", "src/ququart_reentry_clock.rs",
                      "src/reversible_modular.rs", "examples/resident_prepared.rs", "vendor/imasm_core/src/flow.rs"]
    bindings = {"source_sha256":digest(source),"operator_sha256":digest(operator),
                "implementation_sha256":{path:digest(REPO/path) for path in implementation}}
    vox_implementation = ["src/fixed_point_word_arithmetic.rs", "src/dialectic_reentry.rs",
                          "src/dialectic_lean.rs", "src/morphism_factor.rs"]
    vox_repo = REPO.parent / "Vox"
    bindings["vox_implementation_sha256"] = {path:digest(vox_repo/path) for path in vox_implementation}
    snapshot = HERE / f"{prefix}.source"
    for path in implementation:
        target = snapshot / path
        target.parent.mkdir(parents=True,exist_ok=True)
        shutil.copy2(REPO/path,target)
    for path in vox_implementation:
        target = snapshot / "vox_dependency" / path
        target.parent.mkdir(parents=True,exist_ok=True)
        shutil.copy2(vox_repo/path,target)
    environment = os.environ.copy()
    environment["RESIDENT_SOURCE_FILE"] = str(source)
    environment["RESIDENT_CIRCUIT_OPERATOR_FILE"] = str(operator)
    environment["RESIDENT_COMPOSED_MODULAR"] = "1" if args.composed else "0"
    environment.pop("RESIDENT_PREPARED_FILE",None)
    started = time.monotonic()
    started_epoch = time.time()
    receipt = {"command":"cargo resident-static", "input_boundary":"N-only source",
               "source_path":str(source),
               "status":"compiling", "started_epoch":started_epoch, "bindings":bindings,
               "modular_operator":"composed-permutation" if args.composed else "nested-pool"}
    receipt_file.write_text(json.dumps(receipt,indent=2)+"\n")
    with log.open("w") as stream:
        process = subprocess.run(["cargo","resident-static"],cwd=REPO,env=environment,
                                 stdout=stream,stderr=subprocess.STDOUT)
    elapsed = time.monotonic()-started
    receipt.update(status="complete" if process.returncode == 0 else "failed",
                   exit_code=process.returncode,cargo_wall_seconds=elapsed)
    receipt_file.write_text(json.dumps(receipt,indent=2)+"\n")
    if process.returncode == 0:
        for path, expected in bindings["implementation_sha256"].items():
            if digest(REPO/path) != expected:
                raise ValueError(f"implementation changed during compilation: {path}")
        for path, expected in bindings["vox_implementation_sha256"].items():
            if digest(vox_repo/path) != expected:
                raise ValueError(f"Vox implementation changed during compilation: {path}")
        executable = HERE / f"{prefix}.static"
        shutil.copy2(REPO/"target/x86_64-unknown-linux-musl/release/examples/resident_prepared",executable)
        receipt["executable_sha256"] = digest(executable)
        candidates = [path for path in (REPO/"target/x86_64-unknown-linux-musl/release/build").glob(
            "g-momonados-*/out/resident_prepared.rs") if path.stat().st_mtime >= started_epoch]
        if len(candidates) != 1:
            raise ValueError("static construction did not identify one fresh preparation image")
        prepared_text = candidates[0].read_text()
        matches = re.findall(r"const BAKED_PREPARATION_NS:u64=(\d+);",prepared_text)
        if len(matches) != 1:
            raise ValueError("prepared construction lost its compilation clock")
        receipt["preparation_nanoseconds"] = int(matches[0])
        receipt["prepared_image_sha256"] = digest(candidates[0])
        ancestry_text = re.findall(r'preparation_sic_json:("(?:\\.|[^"\\])*")',prepared_text)
        if len(ancestry_text) != 1:
            raise ValueError("prepared construction lost its circuit/frame ancestry")
        ancestry = json.loads(json.loads(ancestry_text[0]))
        for name in ["forward_audit_frame","reverse_audit_frame","prepared_operator_frame"]:
            frame = ancestry["circuit_braid"][name]
            if not frame["exact_banked_return"] or not frame["information_arm_loss_detected"]:
                raise ValueError(f"compiled frame lost its exact return: {name}")
        (HERE/f"{prefix}.preparation_ancestry.json").write_text(json.dumps(ancestry,indent=2)+"\n")
        inspection = subprocess.run(["readelf","-l","-d",str(executable)],capture_output=True,text=True,check=True)
        (HERE/f"{prefix}.elf-inspection.txt").write_text(inspection.stdout)
        receipt["static_elf_verified"] = "INTERP" not in inspection.stdout and "(NEEDED)" not in inspection.stdout
    receipt_file.write_text(json.dumps(receipt,indent=2)+"\n")
    print(json.dumps(receipt))
    raise SystemExit(process.returncode)


if __name__ == "__main__":
    main()
