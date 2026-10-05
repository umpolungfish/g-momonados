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
    parser.add_argument("--stop-after-seconds", type=int, default=85,
                        help="stop the debugger after this many seconds")
    args = parser.parse_args()
    if args.stop_after_seconds < 1:
        parser.error("--stop-after-seconds must be positive")
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
    paths[".gdb"].write_text(f"""set pagination off
set confirm off
set disable-randomization off
set $ququart_stages = 0
break <g_momonados::ququart_folded_work::QuquartFoldedWorkDevice as g_momonados::ququart_factor::QuquartPhaseDevice>::controlled_multiply
commands
silent
set $ququart_stages = $ququart_stages + 1
printf "ququart_controlled_stage_entered=%d\\n", $ququart_stages
continue
end
python
import os, signal, threading, time
def stop_at_limit():
    time.sleep({args.stop_after_seconds})
    os.kill(os.getpid(), signal.SIGINT)
threading.Thread(target=stop_at_limit,daemon=True).start()
end
run
printf "terminal_or_cutoff_debug_stop\\n"
info program
python
if gdb.selected_inferior().pid:
    gdb.execute("info proc status")
    gdb.execute("info registers rip")
    gdb.execute("info proc mappings")
    gdb.execute("bt")
    gdb.execute("kill")
end
quit
""")
    started = time.monotonic()
    with paths[".stdout"].open("w") as out, paths[".stderr"].open("w") as err:
        result = subprocess.run(["timeout", "--signal=TERM", "--kill-after=2s",
                                 f"{args.stop_after_seconds + 3}s",
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
             "elapsed_seconds": elapsed,
             "closure_time_limit_seconds": args.stop_after_seconds + 5,
             "factor_report_present": report is not None,
             "inferior_killed": "killed]" in output,
             "factor_extraction_verified": False}
    state["ququart_controlled_stage_entries"] = [int(line.split("=", 1)[1])
        for line in output.splitlines() if line.startswith("ququart_controlled_stage_entered=")]
    if report is not None:
        verification = subprocess.run([str(case / "verify_readout"), str(case / "prepared.json"),
                                       str(paths[".terminal.stdout"])], text=True, capture_output=True)
        paths[".verification.log"].write_text(verification.stdout + verification.stderr)
        state["factor_extraction_verified"] = verification.returncode == 0
    paths[".json"].write_text(json.dumps(state, indent=2) + "\n")
    print(json.dumps(state))


if __name__ == "__main__":
    main()
