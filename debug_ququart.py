"""Run one prepared membrane under a bounded debugger, retaining its stopped stack."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("case", type=Path)
    parser.add_argument("output_prefix", type=Path)
    args = parser.parse_args()
    case = args.case.resolve()
    prefix = args.output_prefix.resolve()
    manifest = json.loads((case / "manifest.json").read_text())
    binary = case / "membrane"
    if hashlib.sha256(binary.read_bytes()).hexdigest() != manifest["sha256"]:
        raise RuntimeError("prepared executable differs from its manifest")
    paths = {suffix: Path(str(prefix) + suffix) for suffix in
             (".gdb", ".stdout", ".stderr", ".json", ".terminal.stdout", ".verification.log")}
    if any(path.exists() for path in paths.values()):
        raise RuntimeError("debug output prefix already exists; preserve the previous reading")
    paths[".gdb"].write_text("""set pagination off
set confirm off
set disable-randomization off
python
import os, signal, threading, time
def stop_at_limit():
    time.sleep(85)
    os.kill(os.getpid(), signal.SIGINT)
threading.Thread(target=stop_at_limit,daemon=True).start()
end
run
printf "terminal_or_cutoff_debug_stop\\n"
info program
info registers rip
bt 32
kill
quit
""")
    started = time.monotonic()
    with paths[".stdout"].open("w") as out, paths[".stderr"].open("w") as err:
        result = subprocess.run(["timeout", "--signal=TERM", "--kill-after=2s", "88s",
                                 "gdb", "--quiet", "--nx", "--batch", "-x",
                                 str(paths[".gdb"]), str(binary)], stdout=out, stderr=err)
    elapsed = time.monotonic() - started
    output = paths[".stdout"].read_text()
    report = None
    marker = "completed ququart factor extraction\n"
    if marker in output:
        lines = []
        for line in output.split(marker, 1)[1].splitlines():
            if "=" not in line:
                break
            lines.append(line)
        report = marker + "\n".join(lines) + "\n"
        paths[".terminal.stdout"].write_text(report)
    state = {"case": str(case), "debugger_exit": result.returncode,
             "elapsed_seconds": elapsed, "closure_time_limit_seconds": 90,
             "factor_report_present": report is not None,
             "inferior_killed": "killed]" in output,
             "factor_extraction_verified": False}
    if report is not None:
        verification = subprocess.run([str(case / "verify_readout"), str(case / "prepared.json"),
                                       str(paths[".terminal.stdout"])], text=True, capture_output=True)
        paths[".verification.log"].write_text(verification.stdout + verification.stderr)
        state["factor_extraction_verified"] = verification.returncode == 0
    paths[".json"].write_text(json.dumps(state, indent=2) + "\n")
    print(json.dumps(state))


if __name__ == "__main__":
    main()
