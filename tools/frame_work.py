#!/usr/bin/env python3
"""frame_work.py — Stage 25: the candidate walk -> type operation, vox-checked.

FRAME_WORK relocates the fuse ∋ so the ∈…∋ frame ENCLOSES the work that follows an
adjacent ∈∋ dyad:

        ∈ ∋ tau   ->   ∈ tau ∋

It is a CANDIDATE, not a fiat rule. Every application is checked against the current
vox surfaces (verdict + pairs) and must meet the ACCEPTANCE CONDITION:

        walk may be N structurally
        type must be T structurally
        type's substantial pair must enclose the work span

The pair serialisation (walk,type) is marks-only, re-entrant:
        ∈ <numeral len(walk)> ∋ <walk> ∈ <numeral len(type)> ∋ <type>
"""
import subprocess, sys, os, re

VOX = os.environ.get("VOX", "vendor/vox/target/debug/vox")


def vox(*a):
    return subprocess.run([VOX, *a], capture_output=True, text=True).stdout


def verdict(w):
    m = re.search(r"verdict\s+([TFBN])", vox("verdict", w))
    return m.group(1) if m else "?"


def pair(w):
    out = vox("pairs", w)
    m = re.search(r"^\s*(\d+)\s+(\d+)\s+(\d+)\s+(yes|no)\s*(.*)$", out, re.M)
    s = re.search(r"(\d+)\s+paired,\s+(\d+)\s+substantial", out)
    if not m:
        return dict(open=None, close=None, span=0, work=False, interior="", substantial=0)
    return dict(open=int(m.group(1)), close=int(m.group(2)), span=int(m.group(3)),
                work=m.group(4) == "yes", interior=m.group(5).strip(),
                substantial=int(s.group(2)) if s else 0)


def frame_work(w):
    """FRAME_WORK: ∈∋tau -> ∈tau∋  (relocate the fuse to enclose the work)."""
    i = w.find("\u2208\u220b")           # ∈ ∋
    if i < 0:
        return None
    return w[:i] + "\u2208" + w[i + 2:] + "\u220b"


def numeral(n):
    return vox("numeral", str(n)).strip()


def serialize_pair(walk, typ):
    return f"\u2208{numeral(len(walk))}\u220b{walk}\u2208{numeral(len(typ))}\u220b{typ}"


# objects whose walk is an adjacent ∈∋ dyad -> FRAME_WORK applies
CANDIDATES = [("EDIT_WORD", "\u2208\u220b\u22a5\u227b\u22a1"),
              ("REDUCE_WORD", "\u2208\u220b\u22a4\u227b\u22a1")]
# objects already typed / frameless -> FRAME_WORK must be INERT (no adjacent ∈∋)
INERT = [("OP_REENTER", "\u22a2\u2208\u227a\u220b\u22a3"),
         ("OP_MEMBRANE", "\u22a2\u2208\u227b\u22c8\u2299\u22a4\u227b\u22c8\u22a5\u227a\u22c8\u229e\u220b\u22a1\u22c8\u2299\u22a3"),
         ("OP_JUDGE", "\u22a2\u2208\u2299\u220b\u22a3")]


def main():
    ok = True
    print("=== FRAME_WORK candidates (adjacent ∈∋) ===")
    for name, walk in CANDIDATES:
        typ = frame_work(walk)
        wv, tv = verdict(walk), verdict(typ)
        p = pair(typ)
        encloses = p["work"] and p["substantial"] >= 1 and p["span"] >= 2
        accept = (wv in ("N", "B")) and tv == "T" and encloses
        ok &= accept
        print(f"{name}")
        print(f"  walk {walk}  verdict={wv}")
        print(f"  type {typ}  verdict={tv}  span={p['span']} work={p['work']} "
              f"substantial={p['substantial']} interior={p['interior']!r}")
        print(f"  pair-wire = {serialize_pair(walk, typ)}")
        print(f"  acceptance: walk∈{{N,B}}={wv in ('N','B')} type==T={tv == 'T'} "
              f"encloses_work={encloses} -> {'PASS' if accept else 'FAIL'}")
    print("=== FRAME_WORK inertness (must not apply) ===")
    for name, w in INERT:
        r = frame_work(w)
        print(f"  {name} {w}: frame_work={'INAPPLICABLE' if r is None else r}  verdict={verdict(w)}")
    print("FRAME_WORK", "PASS" if ok else "FAIL")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
