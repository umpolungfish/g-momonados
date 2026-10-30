"""Measure native numeral words in the existing nested CUDA interpreter.

This is an input-retention control, not a factorization implementation.
The executable generates the words; this harness only submits and compares them.
"""
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parent
BIN = ROOT / "target/debug/g-momonados"


def run(commands):
    result = subprocess.run(
        [str(BIN)], input="\n".join([*commands, "quit", ""]),
        text=True, capture_output=True, cwd=ROOT, check=True, timeout=60,
    )
    return result.stdout + result.stderr


def main():
    values = [9, 11, 13, 15]
    inputs = run([f"native_numeral encode {n}" for n in values])
    words = re.findall(r"word\s*:\s*([^\s]+)", inputs)
    assert len(words) == len(values) and len(set(words)) == len(values)
    output = run([f"gpu_kernel run_nested {word}" for word in words])
    (ROOT / "measurements/nested_selector_input_control.log").write_text(inputs + output)
    states = re.findall(
        r"gpu_kernel run_nested on device:.*?matches CPU kernel: (?:true|false)",
        output, flags=re.S,
    )
    assert len(states) == len(values), output
    for n, state in zip(values, states):
        print(f"input={n}\n{state}")
        assert "matches CPU kernel: true" in state, f"CPU/CUDA disagreement for {n}"
    print(f"distinct words={len(set(words))}; distinct executed states={len(set(states))}")


if __name__ == "__main__":
    main()
