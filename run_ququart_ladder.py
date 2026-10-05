"""Prepare and execute a certified source ladder, stopping to debug the first unclosed case.

Usage: python3 run_ququart_ladder.py LADDER_DIRECTORY
All source inputs passed to preparation are canonical numeral words.
Reference factors are consulted only after terminal execution and verification.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parent


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("ladder_directory", type=Path)
    args = parser.parse_args()
    directory = args.ladder_directory.resolve()
    ladder = json.loads((directory / "ladder.json").read_text())
    result_path = directory / "executions.json"
    if result_path.exists():
        raise RuntimeError("execution record already exists; preserve it")
    results = []
    for entry in ladder["sources"]:
        bits = entry["bits"]
        record = directory / str(bits)
        case = ROOT / entry["case"]
        if not (case / "manifest.json").exists():
            if case.exists():
                raise RuntimeError("incomplete preparation exists; inspect its live handle or error")
            with (record / "preparation.log").open("w") as log:
                subprocess.run([
                    sys.executable, str(ROOT / "prepare_ququart.py"),
                    "@" + str(record / "source.imasm"), str(case),
                    "--base", "@" + str(directory / "binary_base.imasm"),
                    "--compiled-report", str(record / "retained_braid_input.json"),
                ], cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
        manifest = json.loads((case / "manifest.json").read_text())
        assert manifest["source_word"] == entry["source_word"]
        prefix = record / "execution"
        subprocess.run([sys.executable, str(ROOT / "debug_ququart.py"), str(case), str(prefix)],
                       cwd=ROOT, check=True)
        result = json.loads(Path(str(prefix) + ".json").read_text())
        result["bits"] = bits
        result["runtime_source_inputs"] = []
        result["reference_factors_entered_preparation"] = False
        prepared = (case / "prepared.json").read_bytes()
        binary = (case / "membrane").read_bytes()
        result["preparation_embedded"] = prepared in binary
        result["binary_manifest_matches"] = hashlib.sha256(binary).hexdigest() == manifest["sha256"]
        if result["factor_extraction_verified"]:
            terminal = Path(str(prefix) + ".terminal.stdout").read_text()
            fields = dict(line.split("=", 1) for line in terminal.splitlines() if "=" in line)
            reference = json.loads((record / "reference_only.json").read_text())
            result["matches_certified_reference"] = sorted([fields["p_word"], fields["q_word"]]) == sorted([
                reference["p_word"], reference["q_word"]])
            result["producing_arm_word"] = fields["producing_arm_word"]
            result["factor_words_absent_from_binary"] = all(fields[key].encode() not in binary
                                                          for key in ("p_word", "q_word"))
            if not result["matches_certified_reference"]:
                raise RuntimeError("verified terminal differs from certified factor pair")
        results.append(result)
        result_path.write_text(json.dumps({"results": results,
                                          "remaining_widths": [item["bits"] for item in ladder["sources"][len(results):]],
                                          "stopped_at_first_unclosed": not result["factor_extraction_verified"]},
                                         ensure_ascii=False, indent=2) + "\n")
        print(json.dumps({"bits": bits, "verified": result["factor_extraction_verified"],
                          "debugger_elapsed_seconds": result["elapsed_seconds"]}), flush=True)
        if not result["factor_extraction_verified"]:
            break


if __name__ == "__main__":
    main()
