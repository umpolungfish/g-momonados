#!/usr/bin/env python3
"""
STAGES 161–170 — NATIVE PROJECTION QUOTIENT CONSOLIDATION

Inputs already landed natively in Stages 151–160:
- six permutations of (T,F,tf)
- six distinct resolved FFUSE ladders
- repeated-clear traffic classes 7,8,9 determined by tf position
- control traffic constant 4
- endpoint constant A

This batch proves exact quotient/decoder consequences only.
It makes NO new native measurement.
"""

from itertools import permutations
from collections import Counter, defaultdict
from math import factorial, log2

ATOMS=("T","F","tf")
LANE={
    "T":frozenset(("T",)),
    "F":frozenset(("F",)),
    "tf":frozenset(("t","f")),
}
ORDER=("T","F","t","f")

def reg_name(s):
    if not s: return "N"
    if s==frozenset(ORDER): return "A"
    return "".join(x for x in ORDER if x in s)

def ladder(p):
    q1,q2,q3=p
    return (
        reg_name(LANE[q3]),
        reg_name(LANE[q2] | LANE[q3]),
        "A",
    )

def repeated_total(p):
    # Native Stage-155 law for this sector.
    return {0:7,1:8,2:9}[p.index("tf")]

def control_total(p):
    return 4

def endpoint(p):
    return "A"

P=list(permutations(ATOMS))

# Native anchor table from Stage 151–160.
NATIVE_LADDER={
    ("T","F","tf"):("tf","Ftf","A"),
    ("T","tf","F"):("F","Ftf","A"),
    ("F","T","tf"):("tf","Ttf","A"),
    ("F","tf","T"):("T","Ttf","A"),
    ("tf","T","F"):("F","TF","A"),
    ("tf","F","T"):("T","TF","A"),
}
NATIVE_TRAFFIC={
    ("T","F","tf"):9,
    ("T","tf","F"):8,
    ("F","T","tf"):9,
    ("F","tf","T"):8,
    ("tf","T","F"):7,
    ("tf","F","T"):7,
}
for p in P:
    assert ladder(p)==NATIVE_LADDER[p]
    assert repeated_total(p)==NATIVE_TRAFFIC[p]
    assert control_total(p)==4
    assert endpoint(p)=="A"

print("STAGE 161 — NATIVE PROJECTION MAPS")
print("="*92)
print("  O = ordered execution word")
print("  L = resolved FFUSE ladder")
print("  R = repeated-clear aggregate traffic")
print("  C = control aggregate traffic")
print("  E = endpoint register")
print("  maps fixed from Stage-151–160 native data")
print("STAGE 161 RESULT : True")
print()

# 162: ladder injective
lad_to_p=defaultdict(list)
for p in P:
    lad_to_p[ladder(p)].append(p)
assert len(lad_to_p)==6
assert all(len(v)==1 for v in lad_to_p.values())

print("STAGE 162 — PROVENANCE-LADDER INJECTIVITY")
print("  L distinguishes all 6 orders")
print("  every ladder fibre has cardinality 1")
print("  therefore O <-> L on this six-word sector")
print("STAGE 162 RESULT : True")
print()

# 163: traffic equivalence iff same tf position
for p in P:
    for q in P:
        assert (repeated_total(p)==repeated_total(q)) == (p.index("tf")==q.index("tf"))

print("STAGE 163 — REPEATED-TRAFFIC QUOTIENT")
print("  R(p)=R(q) iff tf occupies the same nesting position")
print("  three traffic classes correspond exactly to tf outer/middle/inner")
print("STAGE 163 RESULT : True")
print()

# 164: T/F swap generates exactly traffic fibres
def swap_TF(p):
    tr={"T":"F","F":"T","tf":"tf"}
    return tuple(tr[x] for x in p)

for p in P:
    q=swap_TF(p)
    assert q!=p
    assert repeated_total(q)==repeated_total(p)
    assert q.index("tf")==p.index("tf")

traffic_fibres=defaultdict(set)
for p in P:
    traffic_fibres[repeated_total(p)].add(p)

for p in P:
    assert traffic_fibres[repeated_total(p)] == {p,swap_TF(p)}

print("STAGE 164 — T/F-SWAP ORBIT THEOREM")
print("  each repeated-traffic fibre is exactly one orbit of T<->F swap")
print("  fibre size = 2 for traffic totals 7, 8, and 9")
print("STAGE 164 RESULT : True")
print()

# 165: equivalence chain
def eq_by(f,p,q): return f(p)==f(q)
for p in P:
    for q in P:
        eqO=(p==q)
        eqL=eq_by(ladder,p,q)
        eqR=eq_by(repeated_total,p,q)
        eqC=eq_by(control_total,p,q)
        eqE=eq_by(endpoint,p,q)
        assert eqO==eqL
        assert (not eqL) or eqR
        assert (not eqR) or eqC
        assert eqC==eqE

print("STAGE 165 — EXACT EQUIVALENCE-RELATION CHAIN")
print("  ~O = ~L  ⊂  ~R  ⊂  ~C = ~E")
print("  cardinalities of quotient sets: 6 -> 6 -> 3 -> 1 -> 1")
print("STAGE 165 RESULT : True")
print()

# 166: fibre cardinalities
sizes_L=sorted(len(v) for v in lad_to_p.values())
sizes_R=sorted(len(v) for v in traffic_fibres.values())
C_fibres=defaultdict(list)
E_fibres=defaultdict(list)
for p in P:
    C_fibres[control_total(p)].append(p)
    E_fibres[endpoint(p)].append(p)
assert sizes_L==[1]*6
assert sizes_R==[2,2,2]
assert sorted(map(len,C_fibres.values()))==[6]
assert sorted(map(len,E_fibres.values()))==[6]

print("STAGE 166 — PROJECTION FIBRE CARDINALITIES")
print("  ladder fibres   : [1,1,1,1,1,1]")
print("  repeated traffic: [2,2,2]")
print("  control traffic : [6]")
print("  endpoint        : [6]")
print("STAGE 166 RESULT : True")
print()

# 167: ladder decoder
LADDER_DECODER={ladder(p):p for p in P}
for p in P:
    assert LADDER_DECODER[ladder(p)]==p

print("STAGE 167 — EXACT ORDER DECODER FROM PROVENANCE")
print("  D_L(L(p))=p for all six native orders")
print("  resolved provenance is lossless on this sector")
print("STAGE 167 RESULT : True")
print()

# 168: traffic decoder
TRAFFIC_DECODER={7:"outer",8:"middle",9:"inner"}
for p in P:
    pos=("outer","middle","inner")[p.index("tf")]
    assert TRAFFIC_DECODER[repeated_total(p)]==pos

print("STAGE 168 — EXACT HEAVY-ATOM POSITION DECODER")
print("  7 -> tf outer")
print("  8 -> tf middle")
print("  9 -> tf inner")
print("  repeated traffic decodes tf position exactly")
print("  AND cannot decode T/F orientation inside that position")
print("STAGE 168 RESULT : True")
print()

# 169: exact uniform information accounting, symbolic and numeric.
H_O=log2(6)
H_L=log2(6)
H_R=log2(3)
H_C=0.0
H_E=0.0
assert abs((H_L-H_R)-1.0)<1e-12
assert abs(H_O-H_L)<1e-12

print("STAGE 169 — UNIFORM INFORMATION ACCOUNTING")
print("  under the uniform distribution on the six execution orders:")
print("  H(O)=H(L)=log2(6)")
print("  H(R)=log2(3)")
print("  H(C)=H(E)=0")
print("  L -> R loses exactly 1 bit: the T/F orientation")
print("  R retains log2(3) bits: the tf nesting position")
print("STAGE 169 RESULT : True")
print()

# 170 synthesis
print("STAGE 170 — NATIVE/ALGEBRAIC BRIDGE CHECKPOINT")
print("  Stage-150 aggregate commutative quotient:")
print("    ordered word -> unordered multiplicity/schedule polynomial")
print("  Stage-160 native refinement:")
print("    ordered word <-> provenance ladder -> traffic quotient -> endpoint")
print()
print("  On {T,F,tf}:")
print("    provenance retains the full order")
print("    repeated traffic = quotient by T/F swap")
print("    control traffic and endpoint erase the entire order fibre")
print()
print("  Therefore:")
print("    aggregate equality does not imply native trace identity")
print("    AND resolved native provenance supplies a lossless section")
print("    of the six-element order fibre in this measured sector.")
print()
print("PARACONSISTENT LANDING")
print("  the same commutative aggregate object supports six native histories")
print("  AND those histories are exactly recoverable from resolved provenance.")
print("  traffic is coarser than provenance")
print("  AND finer than endpoint semantics.")
print()
print("STAGE 170 RESULT : True")
print()
print("BATCH 161–170 RESULT : True")
