#!/usr/bin/env python3
"""
STAGE 104 — RESOLVED WORD LIFT / ABELIANIZED TRAFFIC PROJECTION

Native anchor: Stage 102/103 four-coordinate weighted cube.

Execution-order optional re-clear generators:
    z1, z2, z3, z4
correspond to reconstructed envelopes with traffic weights
    1, 2, 3, 4.

A schedule bit-vector b=(b1,b2,b3,b4) determines the ordered subword

    w(b) = z1^b1 z2^b2 z3^b3 z4^b4

in the fixed execution order.

The aggregate projection forgets word position and keeps only total weight:

    ab(z_g) = x^g
    ab(w(b)) = x^{tau(b)}
    tau(b) = sum_i b_i g_i.

Thus different resolved words can have the same aggregate monomial:
    z3      -> x^3
    z1 z2   -> x^3

This is the exact algebraic shape of the Stage-103 collision separation:
resolved word distinct, aggregate traffic equal.

Important:
  This stage does NOT claim that native execution freely permutes generators.
  It uses a free-word lift as a mathematical representation of the
  already-measured fixed-order schedule code.
"""

from itertools import product
from collections import defaultdict

weights = (1,2,3,4)
names = ("z1","z2","z3","z4")

def word(bits):
    return tuple(name for bit,name in zip(bits,names) if bit)

def tau(bits):
    return sum(bit*g for bit,g in zip(bits,weights))

cube = list(product((0,1), repeat=4))
words = {b:word(b) for b in cube}

# Resolved schedule code is injective.
assert len(set(words.values())) == 16

# Aggregate weighted projection.
fibres = defaultdict(list)
for b in cube:
    fibres[tau(b)].append(b)

assert [len(fibres[k]) for k in range(11)] == [1,1,1,2,2,2,2,2,1,1,1]

# Stage-103 collision identities.
expected = {
    3:{(0,0,1,0),(1,1,0,0)},
    4:{(0,0,0,1),(1,0,1,0)},
    5:{(0,1,1,0),(1,0,0,1)},
    6:{(0,1,0,1),(1,1,1,0)},
    7:{(0,0,1,1),(1,1,0,1)},
}
for k,bs in expected.items():
    assert set(fibres[k]) == bs

# Abelianization is multiplicative on concatenated abstract words
# when weight is extended additively.
def wt_word(w):
    return sum(int(z[1:]) for z in w)

abstract_pairs = [
    (("z1",),("z2",)),
    (("z3",),("z4",)),
    (("z1","z4"),("z2","z3")),
]
for u,v in abstract_pairs:
    assert wt_word(u+v) == wt_word(u)+wt_word(v)

print("STAGE 104 — RESOLVED WORD LIFT / ABELIANIZED TRAFFIC PROJECTION")
print("="*88)
print("104A resolved schedule code:")
print("  w(b)=z1^b1 z2^b2 z3^b3 z4^b4")
print("  fixed execution order; 16 distinct words")
print()
print("104B weighted abelianization:")
print("  ab(z_g)=x^g")
print("  ab(w)=x^tau")
print()
print("104C native collision lift:")
for k in range(3,8):
    print(f"  tau={k}:")
    for b in fibres[k]:
        print("    ", b, "->", " ".join(words[b]) or "1", f"-> x^{k}")
print()
print("104D information hierarchy:")
print("  resolved word remembers selected coordinate positions")
print("  aggregate monomial remembers only weighted sum")
print("  defect and endpoint forget the weighted sum as well")
print()
print("104E algebraic distinction:")
print("  word multiplication is order-sensitive in the free-word lift")
print("  weighted monomials commute after abelianization")
print()
print("104F qualification:")
print("  no claim that native frame execution permits arbitrary generator")
print("  permutations; the word lift represents the measured fixed-order code.")
print()
print("PARACONSISTENT LANDING")
print("  resolved execution is represented by a finer word object")
print("  AND aggregate traffic is its commutative weighted projection.")
print()
print("STAGE 104 RESULT : True")
