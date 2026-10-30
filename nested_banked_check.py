"""Check combo2's banking controls inside the existing nested CUDA entry."""
from pathlib import Path
import os
import re
import subprocess

ROOT = Path(__file__).resolve().parent
WORD = "⊢⊙∈≻⊤≺⊥⋈⊞∋⊡⊣"
words = [WORD[k:] + WORD[:k] for k in range(len(WORD))]
words += ["⊢∈∈⊤∋≺∋⊡⊣", "⊢∈⊤∋≺⊡⊣", "⊢∈⊤⊤≺∋⊡⊣",
          "⊢∈⊤∋∈⊥∋⊡⊣", "⊢∈⊙∋⊡⊣", "⊢⊡⊤⊙≺∋⊣",
          "⊢" + "∈"*80 + "⊤" + "∋"*80 + "⊡⊣", "⊙∈∋⊙"]
commands = [f"gpu_kernel run_nested {word}" for word in words]
commands += ["gpu_kernel verify_nested 4096 20260910", "gpu_kernel verify 512 20260910"]
log = ROOT / "measurements/nested_banked_controls.log"
with log.open("w") as stream:
    subprocess.run([str(ROOT / "target" / os.environ.get("PROFILE", "release") / "g-momonados")],
                   input="\n".join([*commands, "quit", ""]), text=True,
                   stdout=stream, stderr=subprocess.STDOUT, cwd=ROOT, check=True, timeout=360)
output = log.read_text()
rows = re.findall(r"banked tri face: landing=(\w+); weights=\[([^\]]+)\]; state=(\w+)", output)
assert len(rows) == len(words), output
expected_landing = ["A"]*3 + ["Ftf"]*4 + ["tf"]*2 + ["T"]*2 + ["A"]
expected_state = ["holds"]*3 + ["exposed"]*2 + ["vacuous"]*6 + ["holds"]
for (landing, weights, state), want_landing, want_state in zip(rows, expected_landing, expected_state):
    assert (landing, state) == (want_landing, want_state), (landing, state, want_landing, want_state)
assert rows[0] == ("A", "1, 1, 1, 1", "holds"), rows[0]
assert rows[12] == ("T", "1, 0, 0, 0", "holds"), rows[12]
assert rows[13] == ("N", "0, 0, 0, 0", "exposed"), rows[13]
assert rows[14] == ("T", "2, 0, 0, 0", "holds"), rows[14]
movement = re.findall(r"cleared weight=(\d+); restored weight=(\d+); seeds=(\d+); open frames=(\d+)", output)
assert movement[0] == ("1", "1", "1", "0"), movement[0]
assert movement[14] == ("2", "2", "0", "0"), movement[14]
assert movement[-1] == ("0", "0", "2", "0"), movement[-1]
assert output.count("banked face matches CPU combo2 semantics: true") == len(words), output
assert output.count("matches CPU kernel: true") == len(words), output
assert "4096 programs, GPU == CPU on every one" in output, output
assert "512 programs, GPU == CPU on every one" in output, output
print(f"PASS: {len(words)} banking words, all 12 supplied rotations, 4096 nested and 512 classical CPU/CUDA controls")
