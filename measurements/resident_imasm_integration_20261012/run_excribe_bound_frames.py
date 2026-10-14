"""Replay the supplied EXCRIBE composition with N-only target/control bindings."""
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parent
WORKSPACE = ROOT.parents[2]
TRANSLATOR = WORKSPACE / "IMSCRIBr"
WORD = "⊢≻⊙∈≻⊤⊥≻⊤⊥⊞≺≻⊤⊥∋⊡≻≺⋈≻⊣"


def plan(source):
    evidence = lambda p, axis, witness: {
        "kind": "evidence", "proposition": p, "axis": axis,
        "outcome": 16, "numerator": "1", "denominator": "16", "witness": witness,
    }
    actions = [
        [{"kind": "retain"}], [{"kind": "exchange", "generators": [1]}],
        [{"kind": "retain"}], [{"kind": "split", "id": "sic_frame"}],
        [{"kind": "exchange", "generators": [2]}],
        [evidence("sic_comp_clean", "support", "sic_comp_support_1")],
        [evidence("sic_comp_clean", "refutation", "sic_comp_refute_1")],
        [{"kind": "exchange", "generators": [3]}],
        [evidence("sic_leak_clean", "support", "sic_leak_support_1")],
        [evidence("sic_leak_clean", "refutation", "sic_leak_refute_1")],
        [{"kind": "engage", "proposition": "sic_comp_clean"}],
        [{"kind": "inverse"}], [{"kind": "exchange", "generators": [4]}],
        [evidence("sic_dual_recon", "support", "sic_dual_support_1")],
        [evidence("sic_dual_recon", "refutation", "sic_dual_refute_1")],
        [{"kind": "rejoin", "id": "sic_frame"}],
        [{"kind": "latch", "id": "sic_fixed"}],
        [{"kind": "exchange", "generators": [5]}], [{"kind": "inverse"}],
        [{"kind": "exchange", "generators": [1, 2]}],
        [{"kind": "exchange", "generators": [3]}], [{"kind": "sic"}],
    ]
    return {"word": WORD, "plan": {
        "backend": "anyon-composition", "source": str(source), "digit": source % 4,
        "steps": [{"i": i, "symbol": symbol, "actions": action}
                  for i, (symbol, action) in enumerate(zip(WORD, actions))],
    }}


def main():
    sources = {
        "control_208": int(json.loads((ROOT / "inputs/htsx_balanced_control_208.provenance.json").read_text())["source_decimal"]),
        "locked_512": int((ROOT / "inputs/fresh_balanced_512.txt").read_text().strip()),
    }
    binary = WORKSPACE / "G-mOMonadOS/target/release/sic-tool"
    for name, source in sources.items():
        if source.bit_length() < 200:
            raise ValueError("source below the current domain")
        for route in ("supplied", "frame_inverse"):
            saved = plan(source)
            if route == "frame_inverse":
                # Return every still-active inner-frame batch, in reverse order.
                saved["plan"]["steps"][15]["actions"][:0] = [
                    {"kind": "inverse"}, {"kind": "inverse"},
                ]
            stem = ROOT / f"excribe_{name}.{route}"
            plan_path = stem.with_suffix(stem.suffix + ".plan.json")
            plan_path.write_text(json.dumps(saved, indent=2, ensure_ascii=False) + "\n")
            started = time.monotonic()
            result = subprocess.run([sys.executable, "-m", "excribe_vox", "--run-plan", str(plan_path)],
                                    cwd=TRANSLATOR, capture_output=True, text=True, timeout=120)
            elapsed = time.monotonic() - started
            stem.with_suffix(stem.suffix + ".stderr").write_text(result.stderr)
            if result.returncode:
                raise RuntimeError(result.stderr)
            report = json.loads(result.stdout)
            report["reproduction"] = {"word": WORD, "plan": str(plan_path),
                "plan_sha256": hashlib.sha256(plan_path.read_bytes()).hexdigest(),
                "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
                "elapsed_seconds": elapsed, "source_bits": source.bit_length(),
                "input_boundary": "N alone; basis digit = N modulo 4; source binds native precision",
                "evidence_policy": "supplied illustrative threshold/outside-carrier outcome"}
            stem.with_suffix(stem.suffix + ".receipt.json").write_text(json.dumps(report, indent=2) + "\n")
            rejoin = next(event for event in report["events"] if event["kind"] == "rejoin")
            print(name, route, json.dumps({"source_return": rejoin["source_return"],
                "SIC_residual": rejoin["reconstruction_residual"],
                "SIC_tolerance": rejoin["reconstruction_tolerance"], "seconds": elapsed}))


if __name__ == "__main__":
    main()
