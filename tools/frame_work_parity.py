#!/usr/bin/env python3
"""frame_work_parity.py — Stage 26. FRAME_WORK: host oracle vs resident, byte-for-byte.

FRAME_WORK:  ∈ ∋ tau  ->  ∈ tau ∋   (relocate the fuse to enclose the work span tau)
tau = work span after the first ADJACENT ∈∋ dyad, up to the enclosing TANCH ⊣ or end.

The resident machine (Vox crate) owns the rewrite; this Python is the ORACLE. We require
byte-for-byte equality. Acceptance (walk in {N,B}; type == T; substantial pair) is EXTERNAL,
decided by Vox -- the transformer does not know N->T.
"""
import subprocess, sys, os, re

GMO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
VOX = os.environ.get("VOX", os.path.join(GMO, "vendor/vox/target/debug/vox"))
RESIDENT = os.environ.get("FRAME_WORK_BIN", os.path.join(GMO, "Vox/target/debug/frame_work_cli"))
FSPLIT, FFUSE, TANCH = "\u2208", "\u220b", "\u22a3"  # in, ni, tanch


def vox(*a):
    return subprocess.run([VOX, *a], capture_output=True, text=True).stdout


def python_frame_work(w):
    i = w.find(FSPLIT + FFUSE)
    if i < 0:
        return None
    j = w.find(TANCH, i + 2)
    if j < 0:
        j = len(w)
    return w[:i] + FSPLIT + w[i + 2:j] + FFUSE + w[j:]


def resident_frame_work(w):
    return subprocess.run([RESIDENT, w], capture_output=True, text=True).stdout.strip()


def verdict(w):
    m = re.search(r"verdict\s+([TFBN])", vox("verdict", w))
    return m.group(1) if m else "?"


def pair(w):
    out = vox("pairs", w)
    m = re.search(r"^\s*(\d+)\s+(\d+)\s+(\d+)\s+(yes|no)\s*(.*)$", out, re.M)
    s = re.search(r"(\d+)\s+paired,\s+(\d+)\s+substantial", out)
    if not m:
        return dict(span=0, work=False, interior="", substantial=0)
    return dict(span=int(m.group(3)), work=m.group(4) == "yes",
                interior=m.group(5).strip(), substantial=int(s.group(2)) if s else 0)


CORPUS = [
    "\u2208\u220b\u22a5\u227b\u22a1",                 # EDIT_WORD  walk
    "\u2208\u220b\u22a4\u227b\u22a1",                 # REDUCE_WORD walk
    "\u22a2\u2208\u220b\u227b\u22a4\u22a3",           # bracketed adjacent dyad
    "\u22a2\u2208\u220b\u227b\u22a4\u227b\u22c8\u2299\u22a1\u22a3",  # longer tau
    "\u22a2\u2208\u227a\u220b\u22a3",                 # OP_REENTER (non-adjacent -> identity)
    "\u22a2\u2208\u2299\u220b\u22a3",                 # OP_JUDGE   (non-adjacent -> identity)
    "\u22a2\u2208\u227b\u22a4\u22a3",                 # OP_SHAPE   (non-adjacent -> identity)
    "\u22a2\u2299\u22a1\u22a3", "\u22a2\u22a3",       # OP_FIX, OP_PRESERVE
    "\u2208\u220b\u227b\u22c8\u2299\u22a4",           # synthetic tau
    "\u2208\u220b\u227b\u22a4\u2208\u227a\u220b",     # nested-ish tau
]


def main():
    ok = True
    print(f"{'word':<22} {'py==resident':<13} {'type':<12} {'verdict':<8} span work subst")
    for w in CORPUS:
        py = python_frame_work(w)
        res = resident_frame_work(w)
        parity = (res == w) if py is None else (res == py)
        typ = "(identity)" if py is None else res
        p = pair(res)
        ok &= parity
        print(f"{w:<22} {str(parity):<13} {typ:<12} {verdict(res):<8} {p['span']:<4} {str(p['work']):<5} {p['substantial']}")
    print("PARITY", "PASS" if ok else "FAIL")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
