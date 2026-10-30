"""Exercise full operand words in both GPU entries against the shared CPU."""
from pathlib import Path
import os
import re
import subprocess

ROOT = Path(__file__).resolve().parent
binary = ROOT / "target" / os.environ.get("PROFILE", "debug") / "g-momonados"

def run(commands):
    result = subprocess.run([str(binary)], input="\n".join([*commands, "quit", ""]),
                            text=True, capture_output=True, cwd=ROOT, check=True, timeout=180)
    return result.stdout + result.stderr

operands = [1000000016000000063, (1 << 73)-1, (1 << 128)-1]
encoded = run([f"native_numeral encode {n}" for n in operands])
words = re.findall(r"word\s*:\s*([^\s]+)", encoded)
assert len(words) == len(operands), encoded
assert all(len(word) > 64 for word in words)
# Working marks after the old boundary must execute. Also exercise the CPU's
# entire supported capacity and a fork/fuse spanning the former boundary.
words += ["⊢" + "⊙"*65 + "≻⊡⊣", "⊢∈" + "⊙"*65 + "≻∋⊣",
          "⊢" + "⊙"*4094 + "⊣"]
commands = [f"gpu_kernel {entry} {word}" for word in words
            for entry in ("run", "run_nested")]
output = run(commands)
(ROOT / "measurements/nested_word_capacity.log").write_text(encoded + output)
states = re.findall(r"gpu_kernel (?:run_programs|run_nested) on device: (\d+) marks\n"
                    r"  halted: (true|false)   ticks: (\d+).*?matches CPU kernel: (true|false)",
                    output, re.S)
assert len(states) == len(commands), output
for word, states_for_word in zip(words, zip(states[::2], states[1::2])):
    for length, halted, ticks, parity in states_for_word:
        assert int(length) == len(word) and int(ticks) == len(word), states_for_word
        assert halted == parity == "true", states_for_word
rejected = run(["gpu_kernel run_nested " + "⊙"*4097])
assert "word longer than 4096 marks" in rejected, rejected
print(f"PASS: {len(commands)} full-word CPU/CUDA controls, including 60/73/128-bit operands and 4096 marks")
