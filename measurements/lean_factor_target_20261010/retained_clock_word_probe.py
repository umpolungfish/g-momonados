"""Read the exact Godel word after applying one certified retained clock."""
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
MEASUREMENTS = ROOT / "measurements"
SOURCE = int((MEASUREMENTS / "lean_factor_target_20261010/source.decimal").read_text())
LOG = MEASUREMENTS / "godel_hex_ordered_word_source_83b66aee65c8ffa8bbae.log"
CLOCKS = json.loads((MEASUREMENTS / "membrane_reentry_20261010/certified_clocks.json").read_text())["clocks"]
clock = next(row for row in CLOCKS if row["retained"] == 9 and row["seed"] == 1)
table = clock["table"]
hex_word = next(line.split(":", 1)[1].strip() for line in LOG.read_text().splitlines()
                if "hex-digit word   :" in line)
motifs = re.findall("⊢[^⊢⊣]*⊣", hex_word)
assert len(motifs) == 224 and "".join(motifs) == hex_word

def state(motif):
    return int("⊤" in motif) | (int("⊥" in motif) << 1) | (int("⊞" in motif) << 2) | (int("∈" in motif) << 3)

def motif(value):
    fork, truth, falsity, info = (bool(value & 8), bool(value & 1), bool(value & 2), bool(value & 4))
    return ("⊢" + ("∈" if fork else "") + ("⊤" if truth else "") + "≻⋈" +
            ("⊥" if falsity else "") + "≺" + ("⋈" if fork else "") +
            ("⊞" if info else "") + ("∋" if fork else "") + "⊙⊡⊣")

source_states = [state(item) for item in motifs]
mapped = [table[label] for label in source_states]
orientation_pairs = []
for state_label in mapped:
    pair = [bool(state_label & 2), bool(state_label & 4)]
    # Each four-flag motif opens four phase slots in the baked radix-four
    # schedule. Keep its certified coordinate orientation across those slots.
    orientation_pairs.extend([pair] * 4)
orientation_pairs.extend(orientation_pairs[:3])
assert len(orientation_pairs) == 899
transformed_hex = "".join(motif(value) for value in mapped)
parts = []
for item in reversed(mapped):
    parts.extend("≻⋈∈⊥∋" if item & bit else "≻⋈∈⊤∋" for bit in (1, 2, 4, 8))
while len(parts) > 1 and parts[-1] == "≻⋈∈⊤∋":
    parts.pop()
native = "⊢" + "".join(parts) + "⊙⊡⊣"
decoded = subprocess.run(["./godel", "decode", native], cwd=ROOT, capture_output=True,
                         text=True, check=True).stdout
transformed = int(next(line.split(None, 1)[1] for line in decoded.splitlines()
                       if line.startswith("value ")))
read = subprocess.run(["./run_cmds.sh", "trilattice_factor read " + str(transformed)],
                      cwd=ROOT, capture_output=True, text=True, timeout=60, check=True)
output = re.sub(r"\x1b\[[0-9;]*m", "", read.stdout)
period_line = next(line.strip() for line in output.splitlines() if line.strip().startswith("period "))
returned = next(line.split(":", 1)[1].strip() for line in output.splitlines()
                if "hex-digit word   :" in line)
returned_motifs = re.findall("⊢[^⊢⊣]*⊣", returned)
assert "".join(returned_motifs) == returned and len(returned_motifs) == 224
assert returned == transformed_hex
report = {
    "original_source": str(SOURCE),
    "retained": 9,
    "seed": 1,
    "clock_table": table,
    "phase_reentry_clock": clock,
    "phase_source_states": source_states,
    "phase_clock_states": mapped,
    "transformed_source": str(transformed),
    "transformed_bits": transformed.bit_length(),
    "motif_count": len(motifs),
    "mapped_support": sorted(set(mapped)),
    "mapped_seam_return": mapped[0] == mapped[-1],
    "phase_orientation_pairs": orientation_pairs,
    "winding_read": period_line,
    "exact_transformed_word_return": True,
}
path = MEASUREMENTS / "lean_factor_target_20261010/retained_clock_word_probe.json"
path.write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps({key: value for key, value in report.items()
                  if key != "phase_orientation_pairs"}, indent=2))
