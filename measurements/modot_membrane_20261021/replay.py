"""Replay the saved deterministic MoDoT operations into a new local directory."""
from pathlib import Path
import argparse
import hashlib
import json
import subprocess

here = Path(__file__).resolve().parent
workspace = here.parents[2]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("output", type=Path)
options = parser.parse_args()
output = options.output.resolve()
if not output.is_relative_to(workspace):
    parser.error("choose an output directory inside the workspace")
output.mkdir(parents=True, exist_ok=False)
receipt = json.loads((here / "receipt.json").read_text())
for record in receipt["runs"]:
    completed = subprocess.run(record["argv"], cwd=workspace, capture_output=True,
                               text=True, timeout=45)
    (output / (record["case"] + ".stdout")).write_text(completed.stdout)
    (output / (record["case"] + ".stderr")).write_text(completed.stderr)
    record["exit"] = completed.returncode
for key, relative in [("binary_sha256", "MoDoT/ask.bin"),
                      ("catalog_sha256", "imscribing_grammar/IG_catalog.json")]:
    receipt[key] = hashlib.sha256((workspace / relative).read_bytes()).hexdigest()
receipt["source_bindings"] = {
    relative: hashlib.sha256((workspace / relative).read_bytes()).hexdigest()
    for relative in receipt["source_bindings"]
}
(output / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
