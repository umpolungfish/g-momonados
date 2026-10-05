"""Compile a retained ququart membrane from canonical IMASM numeral words.

Supply the source and numeric options as words, or use @path to read a word
from a local file. --base supplies the binary modular base; preparation scales
it by log2(radix) compositions before baking the modular powers. The executable retains joint work amplitudes
through phase measurements and emits factor words after Gödel closure.
"""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parent

def run(args, **kwargs):
    return subprocess.run(args, cwd=ROOT, check=True, text=True,
                          stdout=subprocess.PIPE, stderr=subprocess.PIPE, **kwargs)


def input_word(raw):
    return Path(raw[1:]).read_text().strip() if raw.startswith("@") else raw


def retained_operator(case, inputs):
    case = case.resolve()
    manifest = json.loads((case / "manifest.json").read_text())
    executable = (case / "membrane").read_bytes()
    if hashlib.sha256(executable).hexdigest() != manifest["sha256"]:
        raise RuntimeError("retained Fourier membrane differs from its manifest")
    raw = (case / "prepared.json").read_bytes()
    if raw not in executable:
        raise RuntimeError("retained operator is not the preparation baked in the membrane")
    prepared = json.loads(raw)
    if any(prepared.get(field) != inputs.get(field)
           for field in ("source_word", "accuracy_word")):
        raise RuntimeError("retained Fourier operator differs from requested source or accuracy")
    run([str(ROOT / "target/release/ququart_verify_readout"), "--validate-prepared",
         str(case / "prepared.json")])
    operator = prepared["prepared_operator"]
    if "controlled_power_words" not in operator or "phase_digits_word" not in operator:
        raise RuntimeError("retained factor operator lacks a prepared controlled-power schedule")
    # The physical Fourier operator depends on source precision and accuracy.
    # Modular powers depend on the newly scaled base and must be regenerated.
    return {key: value for key, value in operator.items()
            if key not in ("controlled_power_words", "phase_digits_word")}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", help="canonical source word, or @word-file")
    parser.add_argument("destination", type=Path)
    parser.add_argument("--base", help="canonical binary modular base word, scaled to --radix, or @word-file")
    for option in ("seed", "accuracy", "sk", "net", "refinement", "radix", "native-arm"):
        parser.add_argument(f"--{option}", help=f"canonical {option} word, or @word-file")
    reports = parser.add_mutually_exclusive_group()
    reports.add_argument("--compiled-report", type=Path,
                        help="retained compiler JSON containing IMASM words; the physical braid is recontracted")
    reports.add_argument("--retained-case", type=Path,
                         help="reuse the exact source-bound Fourier operator embedded in a retained membrane")
    args = parser.parse_args()
    case = args.destination.resolve()
    case.mkdir(parents=True, exist_ok=False)
    build = run(["cargo", "build", "--release", "--bin", "ququart_prepare_operator",
                 "--bin", "ququart_verify_readout"])
    (case / "operator_build.log").write_text(build.stdout + build.stderr)
    defaults = json.loads(run([str(ROOT / "target/release/ququart_prepare_operator"), "--defaults"]).stdout)
    inputs = dict(defaults, source_word=input_word(args.source))
    for option in ("seed", "accuracy", "sk", "net", "refinement", "radix", "native_arm"):
        if getattr(args, option) is not None:
            inputs[f"{option}_word"] = input_word(getattr(args, option))
    if args.base is not None:
        inputs["base_word"] = input_word(args.base)
    inputs_path = case / "inputs.json"
    inputs_path.write_text(json.dumps(inputs, ensure_ascii=False, indent=2) + "\n")
    if args.base is not None:
        scaling = run([str(ROOT / "target/release/ququart_prepare_operator"),
                       "--scale-base", str(inputs_path)])
        inputs.update(json.loads(scaling.stdout))
        inputs_path.write_text(json.dumps(inputs, ensure_ascii=False, indent=2) + "\n")
    validation = run([str(ROOT / "target/release/ququart_verify_readout"),
                      "--validate-prepared", str(inputs_path)])
    (case / "input_validation.log").write_text(validation.stdout + validation.stderr)
    run([str(ROOT / "target/release/ququart_prepare_operator"), "--validate-inputs", str(inputs_path)])
    source_word = inputs["source_word"]
    prepared = {"component": "ququart_fourier", "source_word": source_word,
                "accuracy_word": inputs["accuracy_word"],
                "telemetry": "terminal_only"}
    if args.retained_case:
        if args.base is None:
            raise RuntimeError("retained factor preparations require a modular base word")
        prepared["prepared_operator"] = retained_operator(args.retained_case, inputs)
    else:
        if args.compiled_report:
            report = args.compiled_report.read_text()
        else:
            compiler_build = run(["cargo", "build", "--release", "--bin", "g-momonados"])
            (case / "compiler_build.log").write_text(compiler_build.stdout + compiler_build.stderr)
            report = run([str(ROOT / "target/release/g-momonados"), "anyon_ququart_word",
                          source_word, inputs["sk_word"], inputs["net_word"], inputs["capacity_word"],
                          inputs["refinement_word"], inputs["accuracy_word"]]).stdout
        compiled = json.loads(report)
        if (compiled.get("component") != "ququart_fourier" or compiled.get("source_word") != source_word
                or compiled.get("inverse_word") != defaults["capacity_word"]):
            raise RuntimeError("preparation requires a source-bound forward Fourier braid in IMASM words")
        prepared["exchange_words"] = compiled["exchange_words"]
        (case / "compiler.json").write_text(json.dumps(compiled, ensure_ascii=False, indent=2) + "\n")
    binary_name = "ququart_baked"
    if args.base is not None:
        prepared.update(component="ququart_factor", base_word=inputs["base_word"],
                        binary_base_word=inputs["binary_base_word"],
                        native_arm_word=inputs["native_arm_word"],
                        seed_word=inputs["seed_word"], radix_word=inputs["radix_word"])
        binary_name = "ququart_factor_baked"
    prepared_path = case / "prepared.json"
    prepared_path.write_text(json.dumps(prepared, ensure_ascii=False) + "\n")
    (case / "source.imasm").write_text(source_word + "\n")
    env = dict(os.environ, QUQUART_PREPARED_FILE=str(prepared_path))
    binary = case / "membrane"
    with (ROOT / "target/ququart-bake.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        run([str(ROOT / "target/release/ququart_verify_readout"), "--validate-prepared", str(prepared_path)])
        if not args.retained_case:
            contraction = run([str(ROOT / "target/release/ququart_prepare_operator"), str(prepared_path)])
            prepared["prepared_operator"] = json.loads(contraction.stdout)
            prepared.pop("exchange_words")
        prepared_path.write_text(json.dumps(prepared, ensure_ascii=False) + "\n")
        if args.base is not None:
            powers = run([str(ROOT / "target/release/ququart_prepare_operator"),
                          "--prepare-powers", str(prepared_path)])
            prepared["prepared_operator"].update(json.loads(powers.stdout))
            prepared_path.write_text(json.dumps(prepared, ensure_ascii=False) + "\n")
            work = run([str(ROOT / "target/release/ququart_prepare_operator"), "--prepare-work", str(prepared_path)])
            prepared["prepared_work"] = json.loads(work.stdout)
            prepared_path.write_text(json.dumps(prepared, ensure_ascii=False) + "\n")
        build = run(["cargo", "build", "--release", "--bin", binary_name,
                     "--bin", "ququart_verify_readout"], env=env)
        (case / "build.log").write_text(build.stdout + build.stderr)
        validation = run([str(ROOT / "target/release/ququart_verify_readout"),
                          "--validate-prepared", str(prepared_path)])
        (case / "prepared_validation.log").write_text(validation.stdout + validation.stderr)
        shutil.copy2(ROOT / "target/release" / binary_name, binary)
        shutil.copy2(ROOT / "target/release/ququart_verify_readout", case / "verify_readout")
    manifest = {"component": prepared["component"], "binary": str(binary),
                "sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
                "verifier_sha256": hashlib.sha256((case / "verify_readout").read_bytes()).hexdigest(),
                "prepared_sha256": hashlib.sha256(prepared_path.read_bytes()).hexdigest(),
                "source_word": source_word, "accuracy_word": prepared["accuracy_word"],
                "runtime_inputs": [], "telemetry": "terminal_only",
                "prepared_values": "canonical_cell_binary_imasm_words"}
    if args.base is not None:
        manifest.update(base_word=prepared["base_word"], seed_word=prepared["seed_word"],
                        binary_base_word=prepared["binary_base_word"],
                        modular_base_scaling="binary_base_power_log2_radix_mod_source",
                        radix_word=prepared["radix_word"],
                        terminal_nesting="vox_product_over_prefix_meets_prefix_over_product",
                        execution_limits=None, extraction="native_ququart_factor_executor",
                        fourier_operator="contracted_physical_fibonacci_braid",
                        fourier_contraction="preparation_time",
                        modular_work_operator="nested_reversible_arithmetic_on_shared_complex_decision_branches",
                        work_wire_layout="interleaved_source_workspace",
                        source_work_radix="baked_radix_word_live_digit_split_fuse",
                        modular_work_preparation="source_bound_operations_baked_as_numeral_words",
                        feedback_operator="fixed_point_winding",
                        sic_inclusion="shared_control_gram_full_frame_and_dual_synthesis_at_each_phase_readout",
                        native_arm_word=prepared["native_arm_word"],
                        nested_arms=["native_factor_engine", "ququart_phase_with_sic_frame"],
                        native_factor_engine="PARI_GP_canonical_IMASM_word_adapter",
                        native_source_budget_seconds=70,
                        native_source_attempts=[{"factorint_flags":6,"budget_seconds":65},
                                                {"factorint_flags":0,"budget_seconds":5}],
                        native_cofactor_budget_seconds=10,
                        producing_arm_words={"native_factor_engine":defaults["capacity_word"],
                                             "ququart_phase_with_sic_frame":defaults["native_arm_word"]},
                        closure_arithmetic="radix_four_paired_numeral_cells",
                        terminal_factors="direct_godel_closure_words",
                        physical_modular_braids_compiled=False)
    if args.retained_case:
        manifest["retained_fourier_case"] = str(args.retained_case.resolve())
        manifest["retained_fourier_sha256"] = json.loads(
            (args.retained_case / "manifest.json").read_text())["sha256"]
    (case / "manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")
    print(binary)


if __name__ == "__main__":
    main()
