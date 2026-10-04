"""Compile a source-bound ququart executable with IMASM-word constants.

Preparation takes a source and creates a retained, input-free executable.
Its execution emits only its terminal result. With --base it retains joint
work amplitudes through phase measurements and attempts factor extraction.
"""
import argparse
import fcntl
import hashlib
import json
import os
import re
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parent
def run(args, **kwargs):
    return subprocess.run(args, cwd=ROOT, check=True, text=True,
                          stdout=subprocess.PIPE, stderr=subprocess.PIPE, **kwargs)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source")
    parser.add_argument("destination", type=Path)
    parser.add_argument("--base", help="bake a factor-extraction membrane with this phase-estimation base")
    parser.add_argument("--seed", type=int, default=1729)
    parser.add_argument("--accuracy", type=int, default=4)
    parser.add_argument("--sk", type=int, default=5)
    parser.add_argument("--net", type=int, default=7)
    parser.add_argument("--refinement", type=int, default=4)
    parser.add_argument("--compiled-report", type=Path,
                        help="use a retained compiler report; source and residuals are rechecked by the baked binary")
    args = parser.parse_args()
    if int(args.source).bit_length() <= 200:
        raise RuntimeError("qualifying preparations require RSA-style unstructured semiprimes over 200 bits")
    case = args.destination.resolve()
    case.mkdir(parents=True, exist_ok=False)
    encoded = run([str(ROOT / "target/release/godel"), "encode", args.source]).stdout
    source_word = next(line.split(None, 1)[1] for line in encoded.splitlines()
                       if line.startswith("word "))
    if args.compiled_report:
        report = args.compiled_report.read_text()
    else:
        report = run([str(ROOT / "target/release/g-momonados"), "anyon_ququart_word",
                      args.source, str(args.sk), str(args.net), "0",
                      str(args.refinement), str(args.accuracy)]).stdout
    if not report.startswith("Z4 Fourier Fibonacci braid\n") or "inverse=false" not in report:
        raise RuntimeError("preparation requires a successfully compiled forward Fourier braid")
    source_bits = re.search(r"source_bits=(\d+)", report)
    if not source_bits or int(source_bits[1]) != int(args.source).bit_length():
        raise RuntimeError("compiler source precision differs from the requested source")
    residuals = [float(re.search(rf"{name}=([\deE+.-]+)", report)[1])
                 for name in ("computational", "leakage", "unitarity")]
    if any(not (0 <= residual <= 2.0 ** -args.accuracy) for residual in residuals):
        raise RuntimeError("compiled braid exceeds the prepared accuracy budget")
    word = [int(g) for g in report.split("\nword=", 1)[1].split()]
    if not word or any(abs(g) not in range(1, 6) for g in word):
        raise RuntimeError("invalid six-strand Fourier braid")
    prepared = {"component": "ququart_fourier", "source_word": source_word,
                "fourier_word": word, "accuracy_bits": args.accuracy,
                "telemetry": "terminal_only"}
    binary_name = "ququart_baked"
    if args.base is not None:
        base_encoded = run([str(ROOT / "target/release/godel"), "encode", args.base]).stdout
        base_word = next(line.split(None, 1)[1] for line in base_encoded.splitlines()
                         if line.startswith("word "))
        if not 0 <= args.seed < 2**64:
            raise RuntimeError("invalid baked measurement seed")
        prepared.update(component="ququart_factor", base_word=base_word,
                        seed=args.seed)
        binary_name = "ququart_factor_baked"
    prepared_path = case / "prepared.json"
    prepared_path.write_text(json.dumps(prepared) + "\n")
    (case / "compiler.log").write_text(report)
    (case / "source.imasm").write_text(source_word + "\n")
    env = dict(os.environ, QUQUART_PREPARED_FILE=str(prepared_path))
    binary = case / "membrane"
    with (ROOT / "target/ququart-bake.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        contraction_build = run(["cargo", "build", "--release", "--bin", "ququart_prepare_operator"])
        (case / "operator_build.log").write_text(contraction_build.stdout + contraction_build.stderr)
        contraction = run([str(ROOT / "target/release/ququart_prepare_operator"), str(prepared_path)])
        prepared["prepared_operator"] = json.loads(contraction.stdout)
        def native_word(value):
            output = run([str(ROOT / "target/release/godel"), "encode", str(value)]).stdout
            return next(line.split(None, 1)[1] for line in output.splitlines() if line.startswith("word "))
        if args.base is not None:
            prepared["seed_word"] = native_word(prepared.pop("seed"))
        prepared["accuracy_word"] = native_word(prepared.pop("accuracy_bits"))
        prepared.pop("fourier_word")
        def assert_native(value, key=None):
            if isinstance(value, dict):
                for field, child in value.items(): assert_native(child, field)
            elif isinstance(value, list):
                if key and key.endswith("_words"):
                    if any(not isinstance(child, str) for child in value):
                        raise RuntimeError(f"baked {key} must contain only IMASM words")
                else:
                    for child in value: assert_native(child, key)
            elif key in ("component", "telemetry") and isinstance(value, str):
                return
            elif key and key.endswith("_word") and isinstance(value, str):
                return
            else:
                raise RuntimeError(f"baked {key or 'root'} is not an IMASM word or approved metadata")
        assert_native(prepared)
        prepared_path.write_text(json.dumps(prepared) + "\n")
        build = run(["cargo", "build", "--release", "--bin", binary_name, "--bin", "ququart_verify_readout"], env=env)
        (case / "build.log").write_text(build.stdout + build.stderr)
        validation = run([str(ROOT / "target/release/ququart_verify_readout"),
                          "--validate-prepared", str(prepared_path)])
        (case / "prepared_validation.log").write_text(validation.stdout + validation.stderr)
        shutil.copy2(ROOT / "target/release" / binary_name, binary)
    manifest = {"component": prepared["component"], "binary": str(binary),
                "sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
                "source_word": source_word, "accuracy_word": prepared["accuracy_word"],
                "runtime_inputs": [], "telemetry": "terminal_only",
                "prepared_values": "canonical_cell_binary_imasm_words"}
    if args.base is not None:
        manifest.update(base_word=prepared["base_word"], seed_word=prepared["seed_word"],
                        execution_limits=None,
                        extraction="native_ququart_factor_executor",
                        fourier_operator="contracted_physical_fibonacci_braid",
                        fourier_contraction="preparation_time",
                        modular_work_operator="reversible_gates_on_shared_complex_decision_branches",
                        feedback_operator="fixed_point_winding",
                        physical_modular_braids_compiled=False)
    (case / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(binary)


if __name__ == "__main__":
    main()
