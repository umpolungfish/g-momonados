#!/usr/bin/env python3
import argparse, re
from collections import defaultdict

CASES = [
    ("SAME_TT",   "⊢∈⊤⊤≺∋⊣",       {"T":2}, "T"),
    ("NEST_TT",   "⊢∈⊤∈⊤≺∋∋⊣",     {"T":1}, "T"),
    ("SAME_TTT",  "⊢∈⊤⊤⊤≺∋⊣",      {"T":3}, "T"),
    ("NEST_TTT",  "⊢∈⊤∈⊤∈⊤≺∋∋∋⊣",  {"T":1}, "T"),
    ("SAME_TF",   "⊢∈⊤⊥≺∋⊣",       {"T":1,"F":1}, "TF"),
    ("NEST_TF",   "⊢∈⊤∈⊥≺∋∋⊣",     {"T":1,"F":1}, "TF"),
    ("SAME_TTF",  "⊢∈⊤⊤⊥≺∋⊣",      {"T":2,"F":1}, "TF"),
    ("NEST_TTF",  "⊢∈⊤⊤∈⊥≺∋∋⊣",    {"T":2,"F":1}, "TF"),
]

def parse_surviving(s):
    s=s.strip()
    if s in ("", "N", "none", "∅"):
        return {}
    out={}
    for token in s.split(","):
        token=token.strip()
        m=re.fullmatch(r"([TFtf])×(\d+)", token)
        if not m:
            raise AssertionError(("unparsed surviving token",token))
        out[m.group(1)]=int(m.group(2))
    return out

def parse_native(txt):
    vox={}
    weight={}

    # Vox blocks.
    chunks=re.split(r"(?=^Word: )", txt, flags=re.M)
    for ch in chunks:
        m=re.match(r"Word: (.+)\n", ch)
        if not m: continue
        word=m.group(1).strip()
        fm=re.search(r"Final register:\s*(\S+)",ch)
        if fm:
            vox[word]=fm.group(1)

    # Weight blocks.
    chunks=re.split(r"(?=^word\s+:\s*)", txt, flags=re.M)
    for ch in chunks:
        m=re.match(r"word\s+:\s*(.+)\n",ch)
        if not m: continue
        word=m.group(1).strip()
        fm=re.search(r"^\s*final\s*:\s*(\S+)",ch,flags=re.M)
        sm=re.search(r"^\s*surviving:\s*(.+)$",ch,flags=re.M)
        if fm and sm:
            weight[word]=(fm.group(1),parse_surviving(sm.group(1)))
    return vox,weight

def stack(mult):
    h=max(mult.values(),default=0)
    return tuple(
        tuple(sorted(k for k,v in mult.items() if v>=t))
        for t in range(1,h+1)
    )

def validate(txt):
    vox,weight=parse_native(txt)
    for name,word,expected_mult,expected_final in CASES:
        assert word in vox,(name,"missing vox",word)
        assert word in weight,(name,"missing weight",word)
        assert vox[word]==expected_final,(name,"vox final",vox[word],expected_final)
        wfinal,mult=weight[word]
        assert wfinal==expected_final,(name,"weight final",wfinal,expected_final)
        assert mult==expected_mult,(name,"surviving",mult,expected_mult)

    d={name:(word,mult,fin) for name,word,mult,fin in CASES}

    print("STAGES 316–330 — NATIVE CARRY / PROVENANCE BOUNDARY")
    print("="*96)
    print("316 same-frame TT: surviving T×2")
    print("317 nested TT: surviving T×1")
    print("318 same-frame TTT: surviving T×3")
    print("319 nested TTT: surviving T×1")
    print("320 same-frame TF: surviving T×1,F×1")
    print("321 nested TF: surviving T×1,F×1")
    print("322 same-frame TTF: surviving T×2,F×1")
    print("323 outer-same-frame T×2 with inner F: surviving T×2,F×1")
    print()
    print("324 same support endpoint AND different multiplicity under framing: TRUE")
    print("325 same-frame TT threshold stack:", stack({"T":2}))
    print("326 nested TT threshold stack:", stack({"T":1}))
    print("327 support projection collapses both TT cases to T AND weight layers distinguish them")
    print("328 native within-frame law: repeated T deposits add multiplicity")
    print("329 native cross-frame law: repeated T across nesting folds by envelope max")
    print("330 checkpoint: additive carry algebra and provenance max algebra coexist at different structural levels")
    print()
    print("PARACONSISTENT LANDING")
    print("  same glyph support endpoint can be identical")
    print("  AND multiplicity provenance can differ by framing.")
    print("  within one frame duplicate weight adds")
    print("  AND across nested frames duplicate weight max-folds.")
    print()
    print("BATCH 316–330 RESULT : True")

def static():
    print("STAGES 316–330 — NATIVE CARRY / PROVENANCE BOUNDARY")
    print("="*96)
    for name,word,mult,final in CASES:
        print(f"{name:10s} {word}  expected final={final} surviving={mult}")
    print()
    print("STATIC PREDICTION ONLY — NATIVE TRANSCRIPT REQUIRED")
    print("BATCH 316–330 RESULT : PENDING NATIVE")

def main():
    ap=argparse.ArgumentParser()
    ap.add_argument("--native")
    args=ap.parse_args()
    if args.native:
        with open(args.native,"r",encoding="utf-8") as fh:
            validate(fh.read())
    else:
        static()

if __name__=="__main__":
    main()
