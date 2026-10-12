"""Close the target's full numeral through its deterministic split/fuse return."""
from hashlib import sha256
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent
SOURCE = (HERE / "source.decimal").read_text().strip()
SOURCE_WORD = (HERE / "source.imasm").read_text().strip()
EMPTY = "≻⋈∈⊤∋"


def run(args):
    result = subprocess.run(args, cwd=ROOT, capture_output=True, text=True, timeout=20)
    if result.returncode:
        raise RuntimeError(result.stdout + result.stderr)
    return result.stdout


def field(output, label):
    return next(line.split(None, 1)[1] for line in output.splitlines()
                if line.startswith(label + " "))


def decode(word):
    return field(run(["./godel", "decode", word]), "value")


def encode(value):
    word = field(run(["./godel", "encode", value]), "word")
    return [word[1:-3][i:i + 5] for i in range(0, len(word[1:-3]), 5)]


def operate(left, operation, right):
    a, b = encode(left), encode(right)
    width = max(2, len(a), len(b))
    joined = decode("⊢" + "".join(a + [EMPTY] * (2 * width - len(a)) + b) + "⊙⊡⊣")
    output = run(["./godel", "frame-op", joined, str(width), "0", operation,
                  str(2 * width), "1"])
    return field(output, "result")


def divide(left, right):
    a, b = encode(left), encode(right)
    width = max(2, len(a), len(b))
    joined = decode("⊢" + "".join(a + [EMPTY] * (2 * width - len(a)) + b) + "⊙⊡⊣")
    output = run(["./godel", "frame-op", joined, str(width), "0", "divmod",
                  str(2 * width), "1"])
    return field(output, "result"), field(output, "remainder")


assert decode(SOURCE_WORD) == SOURCE
cross = run(["target/release/g-momonados", "factor_membrane", "cross", SOURCE_WORD])
words = {}
for label in ("Λ(W).p", "Λ(W).q", "μ(p,q)"):
    words[label] = next(line.split("=", 1)[1].strip() for line in cross.splitlines()
                        if line.startswith(label + "="))
values = {label: decode(word) for label, word in words.items()}
left, right, fused = values["Λ(W).p"], values["Λ(W).q"], values["μ(p,q)"]
certificate = run(["./godel", "product", SOURCE, left, right])
if "relation.exact-product     PASS" in certificate:
    residual_sign, residual = "+", "0"
else:
    try:
        residual = operate(SOURCE, "sub", fused)
        residual_sign = "+"
    except RuntimeError:
        residual = operate(fused, "sub", SOURCE)
        residual_sign = "-"
assert operate(fused, "add" if residual_sign == "+" else "sub", residual) == SOURCE
residue_mod_p = divide(SOURCE, left)[1]
residue_mod_q = divide(SOURCE, right)[1]
residual_mod_p = divide(residual, left)[1]
residual_mod_q = divide(residual, right)[1]
assert residue_mod_p == residual_mod_p
assert residue_mod_q == residual_mod_q
residue_read = run(["./run_cmds.sh", "trilattice_factor read " + residual])
residue_read = re.sub(r"\x1b\[[0-9;]*m", "", residue_read)
period_line = next(line.strip() for line in residue_read.splitlines()
                   if line.strip().startswith("period "))
hex_word = next(line.split(":", 1)[1].strip() for line in residue_read.splitlines()
                if "hex-digit word   :" in line)
motifs = re.findall("⊢[^⊢⊣]*⊣", hex_word)

report = {
    "source_sha256": sha256(SOURCE.encode()).hexdigest(),
    "source": SOURCE,
    "split_p": left,
    "split_q": right,
    "fused_product": fused,
    "signed_residual": residual_sign + residual,
    "factor_product_certificate": "PASS" if "relation.exact-product     PASS" in certificate else "FAIL",
    "split_fuse_roundtrip": "PASS" if "Γ(Λ(W))==W: true" in cross else "FAIL",
    "source_mod_split_p": residue_mod_p,
    "residual_mod_split_p": residual_mod_p,
    "source_mod_split_q": residue_mod_q,
    "residual_mod_split_q": residual_mod_q,
    "residue_winding": period_line,
    "residue_motif_count": len(motifs),
    "residue_seam_return": bool(motifs and motifs[0] == motifs[-1]),
}
(HERE / "reentry_residue_probe.json").write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps({key: report[key] for key in report if key != "source"}, indent=2))
