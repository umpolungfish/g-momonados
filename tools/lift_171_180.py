#!/usr/bin/env python3
"""
STAGES 171–180 — FORMAL PROJECTION SPECIFICATION / CHECKPOINT

This batch formalizes the finite six-order sector already USER-RUN LANDED
in Stages 151–170. It makes no new native-machine measurement.

Objects:
  O : six ordered executions of {T,F,tf}
  L : six resolved provenance ladders
  P : tf nesting position {outer,middle,inner}
  R : repeated-clear traffic {7,8,9}
  C : control traffic singleton {4}
  E : endpoint singleton {A}
  A : fixed commutative norm aggregate (1+x)^2(1+x^2)

Exact structure:
  O <-> L
  O -> P <-> R
  O -> C
  O -> E
  O -> A

The R-fibres are exactly T/F-swap orbits.
"""

from itertools import permutations
from collections import defaultdict

ATOMS=("T","F","tf")
ORDERS=list(permutations(ATOMS))

LANES={
    "T":frozenset(("T",)),
    "F":frozenset(("F",)),
    "tf":frozenset(("t","f")),
}
DISPLAY=("T","F","t","f")

def reg_name(s):
    if not s:
        return "N"
    if s==frozenset(DISPLAY):
        return "A"
    return "".join(x for x in DISPLAY if x in s)

def ladder(p):
    a,b,c=p
    return (
        reg_name(LANES[c]),
        reg_name(LANES[b] | LANES[c]),
        "A",
    )

def tf_position(p):
    return ("outer","middle","inner")[p.index("tf")]

def repeated_traffic(p):
    return {"outer":7,"middle":8,"inner":9}[tf_position(p)]

def control_traffic(_p):
    return 4

def endpoint(_p):
    return "A"

def norm_aggregate(_p):
    return (1,1,2)   # multiset of lane norms; polynomial (1+x)^2(1+x^2)

def swap_tf_orientation(p):
    tr={"T":"F","F":"T","tf":"tf"}
    return tuple(tr[x] for x in p)

# Native table fixed by Stages 151–160.
NATIVE_LADDERS={
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

for p in ORDERS:
    assert ladder(p)==NATIVE_LADDERS[p]
    assert repeated_traffic(p)==NATIVE_TRAFFIC[p]

print("STAGE 171 — FINITE FORMAL OBJECTS")
print("="*94)
print("  |O|=6 ordered executions")
print("  |L|=6 provenance ladders")
print("  |P|=3 tf positions")
print("  |R|=3 repeated-traffic values")
print("  |C|=|E|=|A|=1 on the measured sector")
print("STAGE 171 RESULT : True")
print()

# 172 O <-> L
L_to_O={ladder(p):p for p in ORDERS}
assert len(L_to_O)==6
for p in ORDERS:
    assert L_to_O[ladder(p)]==p

print("STAGE 172 — PROVENANCE BIJECTION")
print("  encode_L : O -> L is bijective")
print("  decode_L(encode_L(o))=o")
print("STAGE 172 RESULT : True")
print()

# 173 P <-> R
P_to_R={"outer":7,"middle":8,"inner":9}
R_to_P={v:k for k,v in P_to_R.items()}
assert len(R_to_P)==3
for p in ORDERS:
    assert R_to_P[repeated_traffic(p)]==tf_position(p)

print("STAGE 173 — POSITION/TRAFFIC BIJECTION")
print("  P={outer,middle,inner} <-> R={7,8,9}")
print("  repeated traffic is exactly a code for tf nesting position")
print("STAGE 173 RESULT : True")
print()

# 174 swap group action
for p in ORDERS:
    q=swap_tf_orientation(p)
    assert swap_tf_orientation(q)==p
    assert q!=p
    assert tf_position(q)==tf_position(p)
    assert repeated_traffic(q)==repeated_traffic(p)
    assert ladder(q)!=ladder(p)

print("STAGE 174 — T/F INVOLUTION ACTION")
print("  s^2=id")
print("  s has no fixed execution order")
print("  traffic and tf-position are s-invariant")
print("  provenance ladder is not s-invariant")
print("STAGE 174 RESULT : True")
print()

# 175 orbit=fibre
fib=defaultdict(set)
for p in ORDERS:
    fib[repeated_traffic(p)].add(p)
for p in ORDERS:
    assert fib[repeated_traffic(p)]=={p,swap_tf_orientation(p)}

print("STAGE 175 — TRAFFIC FIBRES ARE EXACT SWAP ORBITS")
print("  O/<T<->F> has exactly 3 classes")
print("  each class has size 2")
print("  O/<swap> <-> P <-> R")
print("STAGE 175 RESULT : True")
print()

# 176 terminal projections
assert {control_traffic(p) for p in ORDERS}=={4}
assert {endpoint(p) for p in ORDERS}=={"A"}
assert {norm_aggregate(p) for p in ORDERS}=={(1,1,2)}

print("STAGE 176 — TERMINAL/COMMUTATIVE COLLAPSES")
print("  control traffic is constant 4")
print("  endpoint is constant A")
print("  norm aggregate is constant {1,1,2}")
print("  each erases the entire six-element order fibre")
print("STAGE 176 RESULT : True")
print()

# 177 factorization
for p in ORDERS:
    assert repeated_traffic(p)==P_to_R[tf_position(p)]

print("STAGE 177 — EXACT FACTORIZATION OF REPEATED TRAFFIC")
print("  O --tf_position--> P --code--> R")
print("  repeated_traffic = code o tf_position")
print("  no additional order information reaches R")
print("STAGE 177 RESULT : True")
print()

# 178 equivalence kernels
def kernel(f):
    pairs=set()
    for a in ORDERS:
        for b in ORDERS:
            if f(a)==f(b):
                pairs.add((a,b))
    return pairs

kL=kernel(ladder)
kP=kernel(tf_position)
kR=kernel(repeated_traffic)
kC=kernel(control_traffic)
kE=kernel(endpoint)
kA=kernel(norm_aggregate)

assert kL=={(p,p) for p in ORDERS}
assert kP==kR
assert kL < kR
assert kR < kC
assert kC==kE==kA

print("STAGE 178 — KERNEL LATTICE OF PROJECTIONS")
print("  ker(L)=identity")
print("  ker(P)=ker(R)=T/F-swap equivalence")
print("  ker(C)=ker(E)=ker(A)=universal relation")
print("  strict chain: ker(L) < ker(R) < ker(E)")
print("STAGE 178 RESULT : True")
print()

# 179 sections/retractions
def ladder_decode(L):
    return L_to_O[L]
def traffic_decode_position(r):
    return R_to_P[r]

for p in ORDERS:
    assert ladder_decode(ladder(p))==p
for pos in ("outer","middle","inner"):
    assert traffic_decode_position(P_to_R[pos])==pos

print("STAGE 179 — SECTIONS AND RETRACTIONS")
print("  provenance admits an exact inverse section back to ordered execution")
print("  traffic admits an exact inverse only to tf-position")
print("  traffic has no canonical inverse to full order without one extra T/F bit")
print("STAGE 179 RESULT : True")
print()

print("STAGE 180 — FORMAL CONSOLIDATION CHECKPOINT")
print("  finite native sector:")
print("    O <-> L")
print("    O -> O/<swap> <-> P <-> R")
print("    O -> C = singleton")
print("    O -> E = singleton")
print("    O -> A = singleton")
print()
print("  exact information boundary:")
print("    provenance = full ordered history")
print("    repeated traffic = tf position")
print("    endpoint/control/norm aggregate = no order information")
print()
print("  formalization artifact:")
print("    Stage171_180.lean generated separately")
print("    Lean status is PENDING KERNEL ELABORATION")
print()
print("PARACONSISTENT LANDING")
print("  one commutative aggregate class contains six executions")
print("  AND native provenance gives six distinct canonical representatives.")
print("  repeated traffic is invariant under T/F swap")
print("  AND still separates the three tf positions.")
print()
print("STAGE 180 RESULT : True")
print()
print("BATCH 171–180 RESULT : True")
