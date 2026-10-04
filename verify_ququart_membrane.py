"""Audit a retained membrane's completed output without running it again."""
import argparse
import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("case", type=Path)
    parser.add_argument("--traced", action="store_true")
    args = parser.parse_args()
    case = args.case.resolve()
    manifest = json.loads((case / "manifest.json").read_text())
    prepared = json.loads((case / "prepared.json").read_text())
    binary_hash = hashlib.sha256((case / "membrane").read_bytes()).hexdigest()
    if binary_hash != manifest["sha256"]:
        raise RuntimeError("executable differs from preparation manifest")
    stem = "traced" if args.traced else "run"
    output = (case / f"{stem}.stdout").read_text()
    if (case / f"{stem}.stderr").read_bytes():
        raise RuntimeError("unexpected stderr")
    result = {"binary_sha256": binary_hash, "factor_extraction": False}
    if output.startswith("completed ququart factor extraction\n"):
        fields = dict(line.split("=", 1) for line in output.splitlines()[1:] if "=" in line)
        if fields["source_word"] != prepared["source_word"] or fields["base_word"] != prepared["base_word"]:
            raise RuntimeError("output differs from prepared source or base")
        replay = subprocess.run([str(ROOT / "target/release/ququart_verify_readout"),
                                 str(case / "prepared.json"), str(case / f"{stem}.stdout")],
                                check=True, text=True, capture_output=True)
        result["measured_phase_closure_replayed"] = True
        result["phase_audit"] = replay.stdout.strip()
        result.update(factor_extraction=True, source_word=fields["source_word"],
                      p_word=fields["p_word"], q_word=fields["q_word"],
                      shots_word=fields["shots_word"],
                      godel_product_verified=fields["godel_product_verified"] == "true")
    elif output == "factor extraction failed\n":
        result["completed_error"] = output.strip()
    else:
        raise RuntimeError("missing terminal report")
    if args.traced:
        trace = (case / "syscalls.log").read_text().splitlines()
        if len(trace) != 1 or not trace[0].startswith("write(1,"):
            raise RuntimeError("trace contains additional output or network syscalls")
        if not trace[0].endswith(f"= {len(output.encode())}"):
            raise RuntimeError("terminal write does not cover the complete report")
        result["single_terminal_write_no_network_sends"] = True
    (case / f"{stem}.verification.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result))

if __name__ == "__main__":
    main()
