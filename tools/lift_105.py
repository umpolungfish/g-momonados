#!/usr/bin/env python3
"""
STAGE 105 — TRAFFIC KERNEL CONGRUENCE / QUOTIENT MONOID

Abstract resolved-word lift:
    M = FreeMonoid(z1,z2,z3,z4)

Weight homomorphism:
    wt : M -> (N,+)
    wt(z_g)=g
    wt(uv)=wt(u)+wt(v)

Kernel equivalence:
    u ~_tau v  iff  wt(u)=wt(v)

Because wt is a monoid homomorphism, ~_tau is a monoid congruence:
    u~v and p~q  =>  up ~ vq.

The quotient M/~_tau is isomorphic to the additive submonoid generated
by {1,2,3,4}; since weight 1 is present, this image is all N.

Native schedule restriction:
    S = {z1^b1 z2^b2 z3^b3 z4^b4 : b_i in {0,1}}
has 16 elements and 11 quotient classes under ~_tau, with fibre sizes
    1,1,1,2,2,2,2,2,1,1,1.

Important qualification:
    the full free-monoid quotient is a mathematical lift.
    Native execution measured only the fixed-order squarefree schedule subset S.
"""

from itertools import product
from collections import defaultdict

gens = {"z1":1, "z2":2, "z3":3, "z4":4}

def wt(word):
    return sum(gens[z] for z in word)

# Homomorphism audit on a finite word sample.
sample = [
    (),
    ("z1",), ("z2",), ("z3",), ("z4",),
    ("z1","z2"), ("z4","z1"), ("z2","z3","z1"),
    ("z4","z4"), ("z3","z2","z1"),
]
for u in sample:
    for v in sample:
        assert wt(u+v) == wt(u) + wt(v)

# Kernel-congruence audit.
pairs = [
    (("z3",), ("z1","z2")),
    (("z4",), ("z1","z3")),
    (("z2","z3"), ("z1","z4")),
]
contexts = [
    ((),()),
    (("z1",),("z2",)),
    (("z4",),("z3","z1")),
]
for u,v in pairs:
    assert wt(u)==wt(v)
    for p,q in contexts:
        # left/right same contexts preserve equivalence
        assert wt(p+u+q) == wt(p+v+q)

# Fixed-order squarefree schedule subset.
bits_all = list(product((0,1), repeat=4))
names = ("z1","z2","z3","z4")
schedule_words = {
    b: tuple(z for bit,z in zip(b,names) if bit)
    for b in bits_all
}
assert len(set(schedule_words.values())) == 16

fibres = defaultdict(list)
for b,w in schedule_words.items():
    fibres[wt(w)].append((b,w))

assert sorted(fibres) == list(range(11))
sizes = [len(fibres[k]) for k in range(11)]
assert sizes == [1,1,1,2,2,2,2,2,1,1,1]

# Quotient image on schedule subset is exactly 0..10.
assert {wt(w) for w in schedule_words.values()} == set(range(11))

# Complement symmetry within schedule subset.
full_weight = 1+2+3+4
for b,w in schedule_words.items():
    bc = tuple(1-x for x in b)
    assert wt(schedule_words[bc]) == full_weight - wt(w)

print("STAGE 105 — TRAFFIC KERNEL CONGRUENCE / QUOTIENT MONOID")
print("="*88)
print("105A weight homomorphism:")
print("  wt(z_g)=g")
print("  wt(uv)=wt(u)+wt(v)")
print("  finite word audit: TRUE")
print()
print("105B kernel relation:")
print("  u ~_tau v  iff  wt(u)=wt(v)")
print("  context/congruence audit: TRUE")
print()
print("105C quotient:")
print("  FreeMonoid(z1,z2,z3,z4) / ~_tau  ≅  (N,+)")
print("  because z1 has weight 1, so the image is all N.")
print()
print("105D measured schedule restriction:")
print("  16 fixed-order squarefree schedule words")
print("  quotient image = {0,1,...,10}")
print("  quotient classes = 11")
print("  fibre sizes =", ",".join(map(str,sizes)))
print()
print("105E native collision classes:")
for k in range(11):
    if len(fibres[k]) > 1:
        ws = [" ".join(w) for _,w in fibres[k]]
        print(f"  tau={k}: " + "  ~  ".join(ws))
print()
print("105F complement involution on schedule subset:")
print("  wt(E^c)=10-wt(E)")
print("  quotient fibres are palindromically paired.")
print()
print("105G qualification:")
print("  full free-monoid congruence is a mathematical lift;")
print("  native measurements ground the fixed-order squarefree subset.")
print()
print("PARACONSISTENT LANDING")
print("  resolved words remain distinct before quotient")
print("  AND equal-weight words become identical after quotient.")
print("  the quotient is algebraically commutative in weight")
print("  AND the native resolved trace remains order/position sensitive.")
print()
print("STAGE 105 RESULT : True")
