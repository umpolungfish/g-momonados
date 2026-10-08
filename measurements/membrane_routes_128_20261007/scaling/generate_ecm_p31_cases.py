#!/usr/bin/env python3
"""Generate 256/512/1048-bit semiprimes with a 31-bit prime factor."""

import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent / "ecm_witness_p31"
VOX = Path(__file__).resolve().parents[4] / "Vox" / "target" / "release" / "vox"
P = 2_147_483_647
cases = {}

for bits in (256, 512, 1048):
    q_bits = bits - P.bit_length()
    while True:
        q_hex = subprocess.check_output(
            ["openssl", "prime", "-generate", "-bits", str(q_bits), "-hex"],
            text=True,
        ).strip()
        q = int(q_hex, 16)
        n = P * q
        if n.bit_length() == bits:
            break
    word = subprocess.check_output([str(VOX), "numeral", str(n)], text=True).strip()
    case_dir = ROOT / str(bits)
    case_dir.mkdir(parents=True, exist_ok=True)
    (case_dir / "source.imasm").write_text(word + "\n")
    cases[str(bits)] = {
        "bits": bits,
        "p": str(P),
        "q": str(q),
        "N": str(n),
        "source_word_file": f"{bits}/source.imasm",
    }

(ROOT / "cases.json").write_text(json.dumps(cases, indent=2) + "\n")
print("wrote canonical IMASM words and factor references for 256/512/1048 bits")
