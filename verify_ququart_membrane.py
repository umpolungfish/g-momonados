"""Audit a retained membrane's completed output without running it again."""
import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent

def decode(word):
    report = subprocess.run([str(ROOT / "target/release/godel"), "decode", word],
                            check=True, text=True, capture_output=True).stdout
    return int(re.search(r"^value\s+(\d+)$", report, re.M)[1])

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
        n, p, q = map(int, (fields["source"], fields["p"], fields["q"]))
        if not (1 < p < n and 1 < q < n and p*q == n):
            raise RuntimeError("invalid proper factor product")
        if decode(prepared["source_word"]) != n or decode(prepared["base_word"]) != int(fields["base"]):
            raise RuntimeError("output differs from prepared source or base")
        if decode(fields["p_word"]) != p or decode(fields["q_word"]) != q:
            raise RuntimeError("factor words differ from numeric factors")
        replay = subprocess.run([str(ROOT / "target/release/ququart_verify_readout"),
                                 str(case / "prepared.json"), str(case / f"{stem}.stdout")],
                                check=True, text=True, capture_output=True)
        result["measured_phase_closure_replayed"] = True
        result["phase_audit"] = replay.stdout.strip()
        result.update(factor_extraction=True, p=str(p), q=str(q), shots=int(fields["shots"]))
    elif output.startswith("completed with error: "):
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
