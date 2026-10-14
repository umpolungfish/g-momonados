"""Wire and execute a completed N-only static banked membrane through Vox."""
import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import signal
import subprocess
import sys
import threading
import time

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[1]


def digest(path):
    result = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            result.update(chunk)
    return result.hexdigest()


def relay_output(pipe, terminal, captured):
    while chunk := os.read(pipe.fileno(), 4096):
        captured.append(chunk)
        terminal.buffer.write(chunk)
        terminal.buffer.flush()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("tag", help="completed build prefix")
    parser.add_argument("--seconds", type=int, choices=(300, 600), default=300)
    parser.add_argument("--memory-gib", type=int, default=28,
                        help="Vox address-space bound, recorded with the execution")
    parser.add_argument("--collect-only", action="store_true",
                        help="verify an already finished run without executing it again")
    parser.add_argument("--small-diagnostic", action="store_true",
                        help="verify an explicitly requested small-source diagnostic")
    args = parser.parse_args()
    if not re.fullmatch(r"[A-Za-z0-9_-]+", args.tag):
        parser.error("invalid build tag")
    if args.memory_gib <= 0:
        parser.error("memory bound must be positive")
    build = json.loads((HERE / f"{args.tag}.static-build.receipt.json").read_text())
    if build.get("exit_code") != 0 or not build.get("static_elf_verified"):
        parser.error("the static build and ELF inspection must finish first")
    binary_tag = f"{args.tag}.static"
    executable = HERE / binary_tag
    source = Path(build.get("source_path", str(HERE / "inputs/fresh_balanced_512.imasm")))
    if digest(executable) != build["executable_sha256"] or digest(source) != build["bindings"]["source_sha256"]:
        parser.error("source or static executable changed after compilation")
    runner = REPO.parent / "Vox/target/release/vox"
    anchor = REPO / "measurements/factor_source_carrier_20261011/dispatch_20261012_resident_glut_anchor.diagrammar.txt"
    tape = HERE / f"{binary_tag}.rewired.imasm"
    wiring_receipt = HERE / f"{binary_tag}.wiring-binding.json"
    if not tape.exists():
        command = [sys.executable, str(REPO / "tools/vox_binary_rewire.py"),
                   "--elf", str(executable), "--vox", str(runner), "--diagrammar", str(anchor),
                   "--output", str(HERE / f"{binary_tag}.rewired.json.gz"),
                   "--diagrammar-output", str(HERE / f"{binary_tag}.rewired.diagrammar.txt.gz"),
                   "--imasm-output", str(tape)]
        with (HERE / f"{binary_tag}.rewire.log").open("x") as stream:
            subprocess.run(command, cwd=REPO, stdout=stream, stderr=subprocess.STDOUT, check=True)
        wiring_receipt.write_text(json.dumps({"executable_sha256": digest(executable),
            "source_sha256": digest(source), "tape_sha256": digest(tape),
            "vox_sha256": digest(runner), "anchor_sha256": digest(anchor)}, indent=2) + "\n")
    wiring = json.loads(wiring_receipt.read_text())
    if any(wiring[key] != digest(path) for key, path in (
        ("executable_sha256", executable), ("source_sha256", source),
        ("tape_sha256", tape), ("vox_sha256", runner), ("anchor_sha256", anchor))):
        parser.error("rewritten image bindings changed")
    execution = f"vox_progress_{args.seconds}"
    stem = HERE / f"{binary_tag}.{execution}"
    state_path = Path(str(stem) + ".supervision.json")
    if args.collect_only:
        state = json.loads(state_path.read_text())
        if state["status"] not in ("finished", "timed_out"):
            parser.error("the supervised run must finish before collecting it")
        collect(binary_tag, source, execution, stem, build, state, args.small_diagnostic)
        return
    if state_path.exists():
        parser.error("this bounded execution already has a record")
    environment = os.environ.copy()
    environment.update(VOX_STEPS="50000000000000", VOX_PROGRESS="1")
    state = {"status": "running", "run_bound_seconds": args.seconds,
             "address_space_limit_bytes":args.memory_gib*1024**3,
             "bindings": wiring, "build_receipt": str(HERE / f"{args.tag}.static-build.receipt.json")}
    state_path.write_text(json.dumps(state, indent=2) + "\n")
    print(json.dumps({"stage": "Vox execution", "seconds": args.seconds}), flush=True)
    started = time.monotonic()
    timed_out = False
    def bound_memory():
        limit=args.memory_gib*1024**3
        resource.setrlimit(resource.RLIMIT_AS,(limit,limit))
    stdout_chunks = []
    stderr_chunks = []
    process = subprocess.Popen([str(runner), "run", str(tape)], cwd=REPO,
        env=environment, stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True,
        preexec_fn=bound_memory)
    stdout_thread = threading.Thread(target=relay_output,
        args=(process.stdout, sys.stdout, stdout_chunks), daemon=True)
    stderr_thread = threading.Thread(target=relay_output,
        args=(process.stderr, sys.stderr, stderr_chunks), daemon=True)
    stdout_thread.start()
    stderr_thread.start()
    state["pid"] = process.pid
    state_path.write_text(json.dumps(state, indent=2) + "\n")
    try:
        code = process.wait(timeout=args.seconds)
    except subprocess.TimeoutExpired:
        timed_out = True
        os.killpg(process.pid, signal.SIGTERM)
        try:
            code = process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            code = process.wait(timeout=5)
    stdout_thread.join()
    stderr_thread.join()
    elapsed = time.monotonic() - started
    exit_status = 124 if timed_out else code
    Path(str(stem) + ".time").write_text(f"wall_seconds {elapsed:.9f}\nexit_status {exit_status}\n")
    state.update(status="timed_out" if timed_out else "finished", returncode=code,
                 exit_status=exit_status, wall_seconds=elapsed)
    state_path.write_text(json.dumps(state, indent=2) + "\n")
    collect(binary_tag, source, execution, stem, build, state, args.small_diagnostic,
            b"".join(stdout_chunks), b"".join(stderr_chunks),
            Path(str(stem) + ".time").read_text())


def collect(binary_tag, source, execution, stem, build, state, small_diagnostic=False,
            stdout_bytes=None, stderr_bytes=None, timing_text=None):
    command = [sys.executable, str(HERE / "verify_monadic_receipt.py"),
               binary_tag, str(source), execution]
    if small_diagnostic:
        command.append("--small-diagnostic")
    observation = None
    if stdout_bytes is not None and stderr_bytes is not None:
        command.append("--observation-stdin")
        observation = json.dumps({"stdout":base64.b64encode(stdout_bytes).decode("ascii"),
            "stderr":base64.b64encode(stderr_bytes).decode("ascii"),"time":timing_text}).encode()
    has_saved_output = (Path(str(stem) + ".stdout").is_file()
                        and Path(str(stem) + ".stderr").is_file())
    if observation is not None or has_saved_output:
        subprocess.run(command, cwd=REPO, input=observation, check=True)
    elif not Path(str(stem) + ".receipt.json").is_file():
        parser.error("this run has no captured output available for verification")
    receipt = json.loads(Path(str(stem) + ".receipt.json").read_text())
    receipt["compilation"] = build
    receipt["supervision"] = state
    Path(str(stem) + ".receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps({"exit_status": state["exit_status"], "verified_factor": receipt["verified_factor"],
                      "wall_seconds": state["wall_seconds"], "divisor": receipt.get("divisor"),
                      "quotient": receipt.get("quotient"), "last_progress": receipt.get("last_progress")}))


if __name__ == "__main__":
    main()
