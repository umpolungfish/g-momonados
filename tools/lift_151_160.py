#!/usr/bin/env python3
"""
STAGES 151–160 — NATIVE ORDER/PROVENANCE BOUNDARY BATCH

Purpose
-------
Test one fixed unordered deposit multiset {T,F,tf} across all six execution
orders, with two native paths:

  control  : one AREV before the three LIFO fuses
  repeated : AREV before every fuse

Native commands are emitted separately in stage_151_160_qr3.txt.

Run:
  qr3 "<contents of stage_151_160_qr3.txt command body>" > stage_151_160_native.txt

or source/paste the supplied qr3 command in your existing shell router.

Then:
  p3 tools/lift_151_160.py --native stage_151_160_native.txt

No native result is claimed unless --native parses an actual qr3 transcript.
"""

import argparse
import re
from collections import Counter
from itertools import permutations

ATOMS=("T","F","tf")
GLYPH={"T":"⊤","F":"⊥","tf":"⊞"}
LANES={
    "T":frozenset(("T",)),
    "F":frozenset(("F",)),
    "tf":frozenset(("t","f")),
}
ORDER=("T","F","t","f")

def reg_name(s):
    if not s:
        return "N"
    if s == frozenset(ORDER):
        return "A"
    return "".join(x for x in ORDER if x in s)

def expected_ladder(p):
    q1,q2,q3=p
    return (
        reg_name(LANES[q3]),
        reg_name(LANES[q2] | LANES[q3]),
        "A",
    )

def make_word(p,repeated):
    a,b,c=(GLYPH[x] for x in p)
    tail="≺∋≺∋≺∋" if repeated else "≺∋∋∋"
    return f"⊢∈{a}∈{b}∈{c}{tail}⊣"

def expected_traffic(p,repeated):
    # Native laws already established by Stages 89–90:
    # control: C=R=||Γ1||=4 because raw W=4 and final envelope=4.
    # repeated: C=R=4+||Γ3||+||Γ2||.
    if not repeated:
        return (4,4)
    q1,q2,q3=p
    g3=len(LANES[q3])
    g2=len(LANES[q2] | LANES[q3])
    x=4+g3+g2
    return (x,x)

CASES={}
for p in permutations(ATOMS):
    CASES[make_word(p,False)]=(p,"control")
    CASES[make_word(p,True)]=(p,"repeated")

STEP_RE=re.compile(r"^\s*\d+\s+(\S+)\s+\S+\s+(\S+)\s+→\s+(\S+)\s*$")
FINAL_VOX_RE=re.compile(r"^\s*Final register:\s*(\S+)")
VERDICT_RE=re.compile(r"^\s*Tri-ancestral verdict:\s*(\S+)")
WEIGHT_SUM_RE=re.compile(
    r"^\s*deposits\s+(\d+)\s+cleared\s+(\d+)\s+restored\s+(\d+)\s+seeded\s+(\d+)\s+inert\s+(\d+)"
)
WEIGHT_FINAL_RE=re.compile(r"^\s*final\s*:\s*(\S+)")

def parse_vox(text):
    out={}
    current=None
    buf=[]
    def flush():
        nonlocal current,buf
        if current is None:
            return
        steps=[]
        final=verdict=None
        for line in buf:
            m=STEP_RE.match(line)
            if m:
                steps.append(m.groups())
            m=FINAL_VOX_RE.match(line)
            if m:
                final=m.group(1)
            m=VERDICT_RE.match(line)
            if m:
                verdict=m.group(1)
        # qr3 may echo duplicate command blocks; retain the richest.
        rec={"steps":steps,"final":final,"verdict":verdict}
        if current not in out or len(steps)>len(out[current]["steps"]):
            out[current]=rec
        current=None; buf=[]
    for line in text.splitlines():
        if line.startswith("Word: "):
            flush()
            current=line.split("Word: ",1)[1].strip()
            buf=[line]
        elif current is not None:
            buf.append(line)
    flush()
    return out

def parse_weight(text):
    out={}
    current=None
    buf=[]
    def flush():
        nonlocal current,buf
        if current is None:
            return
        final=None
        summary=None
        moves=[]
        for line in buf:
            m=WEIGHT_FINAL_RE.match(line)
            if m:
                final=m.group(1)
            m=WEIGHT_SUM_RE.match(line)
            if m:
                summary=tuple(map(int,m.groups()))
            if "CLEAR loses " in line or "fuse restores " in line:
                moves.append(line.strip())
        rec={"final":final,"summary":summary,"moves":moves}
        # Prefer blocks carrying a summary.
        if current not in out or (summary is not None and out[current]["summary"] is None):
            out[current]=rec
        current=None; buf=[]
    for line in text.splitlines():
        m=re.match(r"^word\s+:\s+(\S+)\s*$",line)
        if m:
            flush()
            current=m.group(1)
            buf=[line]
        elif current is not None:
            buf.append(line)
    flush()
    return out

def fuse_ladder(vox_rec):
    return tuple(after for glyph,before,after in vox_rec["steps"] if glyph=="∋")

def static_audit():
    assert len(CASES)==12
    rows=[]
    for p in permutations(ATOMS):
        ctl=make_word(p,False)
        rep=make_word(p,True)
        lad=expected_ladder(p)
        tc=expected_traffic(p,False)
        tr=expected_traffic(p,True)
        rows.append((p,lad,tc,tr))
    assert len({r[1] for r in rows})==6
    assert sorted(r[3][0] for r in rows)==[7,7,8,8,9,9]
    assert {r[2] for r in rows}=={(4,4)}
    return rows

def print_static(rows):
    print("STAGES 151–160 — NATIVE ORDER/PROVENANCE BOUNDARY BATCH")
    print("="*94)
    print("151 fixed unordered native sector:")
    print("  deposit multiset {T,F,tf}; all 6 S3 permutations")
    print("  lane norms: |T|=1, |F|=1, |tf|=2")
    print("  commutative norm polynomial: (1+x)^2(1+x^2)")
    print()
    print("152 order fibre:")
    print("  3 distinct deposit labels -> 3! = 6 ordered native words")
    print()
    print("153 Stage-82 suffix-ladder predictions:")
    for p,lad,tc,tr in rows:
        print(f"  {p} -> ladder {lad}")
    print("  six predicted ladder classes: TRUE")
    print()
    print("154 control path prediction:")
    print("  one AREV before all pops")
    print("  all six: aggregate traffic C/R = 4/4")
    print("  endpoint A for all six")
    print()
    print("155 repeated-clear prediction:")
    for p,lad,tc,tr in rows:
        print(f"  {p}: C/R={tr[0]}/{tr[1]}")
    print("  aggregate classes: 7/7, 8/8, 9/9; each occurs twice")
    print()
    print("156 projection prediction:")
    print("  ordered word: 6 classes")
    print("  resolved FFUSE ladder: 6 classes")
    print("  repeated aggregate traffic: 3 classes")
    print("  control aggregate traffic: 1 class")
    print("  endpoint: 1 class (A)")
    print()
    print("157 path distinction:")
    print("  same permutation has same predicted provenance ladder AND endpoint")
    print("  while repeated AREV changes aggregate exposure traffic from 4/4 to 7–9/7–9")
    print()
    print("158 symmetry prediction:")
    print("  repeated aggregate traffic forgets T/F order")
    print("  AND retains the nesting position of the two-lane atom tf:")
    print("    tf outer -> 7, tf middle -> 8, tf inner -> 9")
    print()
    print("159 native theorem status: PENDING TRANSCRIPT")
    print("160 checkpoint status: PENDING NATIVE PARSE")
    print()
    print("STATIC/EXECUTABLE PRECHECK : True")

def validate_native(text):
    rows=static_audit()
    vox=parse_vox(text)
    wt=parse_weight(text)

    missing_vox=sorted(w for w in CASES if w not in vox)
    missing_wt=sorted(w for w in CASES if w not in wt)
    if missing_vox or missing_wt:
        raise AssertionError(
            f"incomplete native transcript: missing vox={len(missing_vox)} weight={len(missing_wt)}"
        )

    obs=[]
    for p in permutations(ATOMS):
        expected=expected_ladder(p)
        for repeated,kind in ((False,"control"),(True,"repeated")):
            w=make_word(p,repeated)
            v=vox[w]
            q=wt[w]
            lad=fuse_ladder(v)
            expC,expR=expected_traffic(p,repeated)

            assert lad==expected, (p,kind,"ladder",lad,expected)
            assert v["final"]=="A", (p,kind,"vox final",v["final"])
            assert q["final"]=="A", (p,kind,"weight final",q["final"])
            assert q["summary"] is not None, (p,kind,"missing weight summary")
            deposits,cleared,restored,seeded,inert=q["summary"]
            # `deposits` counts deposit OPCODE EVENTS, not lane multiplicity.
            # This batch executes exactly three deposit glyphs: ⊤, ⊥, ⊞.
            # Their lane weight is 1+1+2=4, which is what the C/R laws use.
            assert deposits==3, (p,kind,"deposit events",deposits)
            assert (cleared,restored)==(expC,expR), (
                p,kind,"traffic",(cleared,restored),(expC,expR)
            )
            obs.append({
                "p":p,"kind":kind,"ladder":lad,
                "traffic":(cleared,restored),
                "verdict":v["verdict"],
            })

    controls=[r for r in obs if r["kind"]=="control"]
    repeated=[r for r in obs if r["kind"]=="repeated"]

    assert len({r["ladder"] for r in controls})==6
    assert len({r["ladder"] for r in repeated})==6
    assert {r["traffic"] for r in controls}=={(4,4)}

    reps=Counter(r["traffic"] for r in repeated)
    assert reps==Counter({(7,7):2,(8,8):2,(9,9):2}), reps

    endpoints={vox[make_word(p,r)]["final"] for p in permutations(ATOMS) for r in (False,True)}
    assert endpoints=={"A"}

    # tf-position is exactly encoded by repeated total.
    for r in repeated:
        pos=r["p"].index("tf") # 0 outer, 1 middle, 2 inner
        expected_total={0:7,1:8,2:9}[pos]
        assert r["traffic"]==(expected_total,expected_total)

    print("STAGES 151–160 — NATIVE ORDER/PROVENANCE BOUNDARY")
    print("="*94)
    print("151 fixed unordered sector: NATIVELY MEASURED")
    print("  all six permutations of {T,F,tf} present")
    print("  3 deposit events per word; total deposited lane weight = 4")
    print()
    print("152 commutative norm projection:")
    print("  same multiset AND same norm polynomial (1+x)^2(1+x^2)")
    print()
    print("153 resolved provenance:")
    for r in controls:
        print(f"  {r['p']} -> {r['ladder']}")
    print("  six distinct ladders: TRUE")
    print()
    print("154 control path:")
    print("  all six C/R=4/4 AND endpoint A")
    print()
    print("155 repeated-clear path:")
    for r in repeated:
        print(f"  {r['p']} -> C/R={r['traffic'][0]}/{r['traffic'][1]}")
    print("  distribution {(7,7):2,(8,8):2,(9,9):2}: TRUE")
    print()
    print("156 native projection hierarchy:")
    print("  order word 6 -> ladder 6 -> repeated aggregate 3 -> endpoint 1")
    print("  control aggregate collapses directly to 1")
    print()
    print("157 path comparison:")
    print("  provenance ladder invariant across the two AREV paths: TRUE")
    print("  endpoint invariant across the two AREV paths: TRUE")
    print("  exposure traffic path-sensitive: TRUE")
    print()
    print("158 T/F symmetry quotient:")
    print("  repeated aggregate traffic forgets T/F order")
    print("  AND records exactly the nesting position of tf")
    print("    outer tf=7, middle tf=8, inner tf=9")
    print()
    print("159 native order theorem:")
    print("  same unordered deposit multiset AND same endpoint")
    print("  BUT resolved native provenance retains all six execution orders.")
    print("  aggregate repeated traffic retains a strict 3-class quotient.")
    print()
    print("160 checkpoint:")
    print("  aggregate commutativity is a genuine quotient, not native trace identity.")
    print("  resolved provenance is strictly finer than aggregate traffic and endpoint semantics.")
    print()
    print("PARACONSISTENT LANDING")
    print("  the unordered/native-norm object is fixed")
    print("  AND execution order remains observable in the provenance ladder.")
    print("  aggregate traffic forgets T/F orientation")
    print("  AND retains the position of the heavier tf atom.")
    print()
    print("BATCH 151–160 RESULT : True")

def main():
    ap=argparse.ArgumentParser()
    ap.add_argument("--native", metavar="FILE",
                    help="parse actual qr3 transcript and land Stages 151–160 natively")
    args=ap.parse_args()
    if args.native:
        with open(args.native,"r",encoding="utf-8") as fh:
            validate_native(fh.read())
    else:
        print_static(static_audit())

if __name__=="__main__":
    main()
