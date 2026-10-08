#!/usr/bin/env python3
"""Resume the requested target using only the WordTape ECM extractor."""
import json
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[1]
TARGET = "233108530344407544527637656910680524145619812480305449042948611968495918245135782867888369318577116418213919268572658314913060672626911354027609793166341626693946596196427744273886601876896313468704059066746903123910748277606548649151920812699309766587514735456594993207"
BIN = REPO / "membranes/target/release/membrane_ecm_extract"
CODEC = REPO / "godel"
B1, B2 = 5000, 50000

def codec(*args):
    return subprocess.run([str(CODEC), *args], text=True, capture_output=True, check=True).stdout

if not (ROOT / "source.imasm").exists():
    report = codec("encode", TARGET)
    word = next(line.split(None, 1)[1] for line in report.splitlines() if line.startswith("word "))
    (ROOT / "source.imasm").write_text(word + "\n")
    (ROOT / "source.decimal").write_text(TARGET + "\n")
    (ROOT / "codec.log").write_text(report)
word = (ROOT / "source.imasm").read_text().strip()
records_path = ROOT / "curves.json"
records = json.loads(records_path.read_text()) if records_path.exists() else []
if any(record.get("factor") for record in records):
    print("factor already retained in curves.json", flush=True)
    raise SystemExit(0)
completed = {record["sigma"] for record in records if record["returncode"] == 0}
for sigma in range(6, 106):
    if sigma in completed:
        continue
    started = time.monotonic()
    run = subprocess.run([str(BIN), word, str(B1), str(B2), "1", str(sigma)], text=True, capture_output=True)
    output = run.stdout + run.stderr
    (ROOT / f"sigma_{sigma}.log").write_text(output)
    factor = next((line.removeprefix("factor=") for line in output.splitlines() if line.startswith("factor=")), None)
    record = dict(sigma=sigma, B1=B1, B2=B2, elapsed_seconds=time.monotonic()-started,
                  returncode=run.returncode, factor=factor, product_closes="product_closes=true" in output)
    records.append(record)
    records_path.write_text(json.dumps(records, indent=2) + "\n")
    print(json.dumps(record), flush=True)
    if run.returncode != 0:
        raise SystemExit(run.returncode)
    if factor:
        (ROOT / "factor.imasm").write_text(factor + "\n")
        (ROOT / "factor_decoded.log").write_text(codec("decode", factor))
        break
