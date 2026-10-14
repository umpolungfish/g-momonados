#!/usr/bin/env python3
"""Verify observations after an N-only static membrane execution."""
import argparse
import base64
import hashlib
import json
import re
from pathlib import Path
import sys

parser = argparse.ArgumentParser()
parser.add_argument("tag")
parser.add_argument("source", type=Path)
parser.add_argument("execution", choices=("native", "vox", "vox_progress", "vox_progress_300", "vox_progress_600"))
parser.add_argument("--small-diagnostic", action="store_true",
                    help="verify an explicitly requested small-source diagnostic")
parser.add_argument("--observation-stdin", action="store_true",
                    help="read streamed stdout, stderr and timing from standard input")
args = parser.parse_args()
base = Path(__file__).resolve().parent
word = args.source.read_text().strip()
assert word.startswith("⊢") and word.endswith("⊙⊡⊣")
cells = word[1:-3]
assert len(cells) % 5 == 0
n = 0
for bit in range(len(cells) // 5):
    cell = cells[5*bit:5*bit+5]
    assert cell[:3] == "≻⋈∈" and cell[-1] == "∋" and cell[3] in "⊤⊥"
    n |= (cell[3] == "⊥") << bit
if n.bit_length()<200 and not args.small_diagnostic:
    parser.error("current verification requires the authorized random near-balanced source of at least 200 bits")

def digest(path):
    value=hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda:stream.read(1024*1024),b""):
            value.update(chunk)
    return value.hexdigest()
stdout = base / f"{args.tag}.{args.execution}.stdout"
stderr = base / f"{args.tag}.{args.execution}.stderr"
timing = base / f"{args.tag}.{args.execution}.time"
if args.observation_stdin:
    observation = json.load(sys.stdin)
    stdout_bytes = base64.b64decode(observation["stdout"])
    stderr_bytes = base64.b64decode(observation["stderr"])
    time = observation["time"]
else:
    stdout_bytes = stdout.read_bytes()
    stderr_bytes = stderr.read_bytes()
    time = timing.read_text()
out = stdout_bytes.decode(errors="replace")
err = stderr_bytes.decode(errors="replace")
receipt = {
    "source": str(n), "source_word": word,
    "source_bits":n.bit_length(),
    "small_diagnostic":args.small_diagnostic,
    "source_sha256": digest(args.source),
    "binary_sha256": digest(base / args.tag),
    "execution": args.execution,
    "process_timing": time.strip(),
    "stdout_sha256": hashlib.sha256(stdout_bytes).hexdigest(),
    "stderr_sha256": hashlib.sha256(stderr_bytes).hexdigest(),
    "verified_factor": False,
}
receipt["stdout_decode_replacements"] = out.count("\ufffd")
receipt["stderr_decode_replacements"] = err.count("\ufffd")
compilation=base / f"{args.tag}.build.time"
if compilation.is_file(): receipt["compilation_timing"]=compilation.read_text().strip()
compiled_artifacts={}
for suffix in ("compiled_tables.rs","compiled_support.bin","preparation.stderr"):
    artifact=base / f"{args.tag}.{suffix}"
    if artifact.is_file(): compiled_artifacts[suffix]=digest(artifact)
if compiled_artifacts: receipt["compiled_artifact_sha256"]=compiled_artifacts
receipt["import_stages"]=re.findall(r"^resident-import (.+)$",err,re.M)
preparation=re.search(r"prepared-before-compile nanoseconds=(\d+)",err)
loading=re.search(r"baked-state-loaded seconds=(\S+)",err)
progress=re.findall(r"resident-trigger elapsed=(\S+) product_rows=(.+) phase=(.+)",err)
operations=re.findall(r"resident-Y-clock contractions_calls_ticks_forward_reverse=(.+)",err)
if preparation: receipt["preparation_nanoseconds"]=preparation[1]
if loading: receipt["baked_loading_seconds"]=loading[1]
if progress: receipt["last_progress"]={"trigger_seconds":progress[-1][0],"product":progress[-1][1],"phase":progress[-1][2]}
if operations: receipt["last_reentry_operations"]=operations[-1]
bindings=re.findall(r"resident-monadic-binds=(\d+) lanes_started=(\d+) recent_lanes=(.+)",err)
if bindings:
    receipt["last_monadic_binding"]={"binds":int(bindings[-1][0]),"lanes_started":int(bindings[-1][1]),"recent_lanes":bindings[-1][2]}
prefix_counts=re.findall(r"resident-digit-prefix calls_zero_returns_words=\((\d+),\s*(\d+),\s*(\d+)\)",err)
if prefix_counts:
    receipt["last_digit_prefix_counts"]=dict(zip(("calls","zero_returns","words_created"),map(int,prefix_counts[-1])))
if args.execution.startswith("vox"):
    tape=base / f"{args.tag}.rewired.imasm"
    receipt["rewritten_tape_sha256"]=digest(tape)
    receipt["vox_runner_sha256"]=digest(base.parents[2]/"Vox/target/release/vox")
    samples=re.findall(r"vox-execution seconds=(\S+) steps=(\d+) pc=(0x[0-9a-f]+)",err)
    if samples:
        receipt["last_vm_sample"]={"seconds":samples[-1][0],"steps":int(samples[-1][1]),"pc":samples[-1][2]}
    ready=re.search(r"vox-loading stage=ready seconds=(\S+)",err)
    if ready:receipt["vox_module_loading_seconds"]=ready[1]
snapshot=base / f"{args.tag}.source"
if snapshot.is_dir():
    receipt["source_snapshot_sha256"]={str(path.relative_to(snapshot)):digest(path) for path in sorted(snapshot.rglob("*")) if path.is_file()}
for field, pattern in {
    "trigger_to_readout_seconds": r"^trigger_to_readout_seconds (\S+)$",
    "trigger_to_verified_divisor_seconds": r"^trigger_to_verified_divisor_seconds (\S+)$",
    "phase_monadic_binds": r"^phase-monadic-binds (\d+)$",
    "closed_resident_banks": r"^closed-resident-banks (\d+)$",
    "producer": r"^producer (\S+)$",
}.items():
    match = re.search(pattern, out, re.M)
    if match:
        receipt[field] = match[1]
phase = re.search(r"^phase-numerator (\d+) phase-denominator (\d+)$", out, re.M)
if phase:
    receipt["phase"] = {"numerator": phase[1], "denominator": phase[2]}
factors = re.search(r"^factor (\d+)\nquotient (\d+)$", out, re.M)
if factors:
    d, q = map(int, factors.groups())
    assert 1 < d < n and n % d == 0 and n // d == q and d*q == n
    assert "exit_status 0" in time
    assert "gamma_delta_closed=true product_closed=true" in out
    before = re.search(r"^resident-Y-clock .*=(.+)$", out, re.M)
    after = re.search(r"^closed-resident-Y-clock .*=(.+)$", out, re.M)
    assert before and after and before[1] == after[1]
    receipt.update(divisor=str(d), quotient=str(q), remainder="0", verified_factor=True,
                   boundary_closed=True, retained_reentry=after[1])
    sic=re.search(r"^SIC return residual=(\S+) tolerance=(\S+) signed_residuals=(.+)$",out,re.M)
    assert sic, "successful closure must retain its signed SIC observation"
    receipt["sic"]={"maximum_residual":sic[1],"tolerance":sic[2],"signed_residuals":sic[3]}
path = base / f"{args.tag}.{args.execution}.receipt.json"
path.write_text(json.dumps(receipt, indent=2, ensure_ascii=False) + "\n")
print(f"{path.name}: verified_factor={receipt['verified_factor']}")
