"""A contained numeral must not fix the selector before its outer fuse."""
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parent
artifact = ROOT.parent / "ob3ect/digital/imasm_word_that_constructs_and_selects_the_compa_9eeb5656/imasm_word_that_constructs_and_selects_the_compa_9eeb5656_ob3ect.json"
commands = [f"gpu_kernel apply {artifact} {n}" for n in (17, 1000000016000000063)]
commands += [f"gpu_kernel apply {artifact} ⊢⊡∈⊤∋⊣"]
result = subprocess.run([str(ROOT / "target/release/g-momonados")],
                        input="\n".join([*commands, "quit", ""]), text=True,
                        capture_output=True, cwd=ROOT, check=True, timeout=90)
output = result.stdout + result.stderr
(ROOT / "measurements/nested_composition_banked.log").write_text(output)
words = re.findall(r"composed word: (\S+)", output)
assert len(words) == 2, output
assert all(word.count("⊡") == 1 and word.endswith("∋⊡⊣") for word in words), words
assert output.count("state=holds") == 2, output
assert output.count("open frames=0") == 2, output
assert output.count("inert=1;") == 2, output
assert output.count("banked face matches CPU combo2 semantics: true") == 2, output
assert output.count("matches CPU kernel: true") == 2, output
assert "payload fixes before its terminal interface" in output, output
print("PASS: both contained numerals fuse before fixation; early internal fixation rejected")
