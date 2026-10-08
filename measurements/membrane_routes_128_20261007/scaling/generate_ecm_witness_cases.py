#!/usr/bin/env python3
"""Prepare >=200-bit semiprimes with one small ECM witness factor."""

import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent
VOX = Path(__file__).resolve().parents[4] / "Vox" / "target" / "release" / "vox"
P = 211
CASES = {}

for target_bits in (256, 512, 1048):
    q_bits = target_bits - P.bit_length()
    while True:
        q_hex = subprocess.check_output(
            ["openssl", "prime", "-generate", "-bits", str(q_bits), "-hex"],
            text=True,
        ).strip()
        q = int(q_hex, 16)
        n = P * q
        if n.bit_length() == target_bits:
            break
    encoded = subprocess.check_output([str(VOX), "numeral", str(n)], text=True).strip()
    case_dir = ROOT / "ecm_witness" / str(target_bits)
    case_dir.mkdir(parents=True, exist_ok=True)
    (case_dir / "source.imasm").write_text(encoded + "\n")
    CASES[str(target_bits)] = {
        "bits": target_bits,
        "p": str(P),
        "q": str(q),
        "N": str(n),
        "source_word_file": f"ecm_witness/{target_bits}/source.imasm",
    }

(ROOT / "ecm_witness_cases.json").write_text(json.dumps(CASES, indent=2) + "\n")
print("prepared IMASM-word ECM witness cases at 256, 512, and 1048 bits")
