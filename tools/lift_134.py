#!/usr/bin/env python3
"""
STAGE 134 — CURRENT/FUTURE CONVOLUTION FACTORIZATION

Let the current aggregate fibre polynomial be

    Q_cur(x)=sum_k q_k x^k,

and let the future append schedule polynomial be

    A_G(x)=prod_i (1+x^{g_i})
          =sum_s a_s x^s.

After applying all future appends,

    Q_final(x)=Q_cur(x) A_G(x).

Coefficientwise,

    q_final(k)=sum_s a_s q_{k-s}.

This is exactly Stage 133.

Hence finite-horizon aggregate evolution factors into:
    current state polynomial
    TIMES
    future schedule polynomial.

Composition is associative and commutative on future blocks:

    A_G A_H = A_{G concatenated H}

and therefore

    (Q_cur A_G) A_H = Q_cur (A_G A_H).

The current q-sequence is Markov-complete for arbitrary future append blocks.
The fibre histogram alone is not (Stage 129), because polynomial
multiplication needs coefficient placement, not only the multiset of coefficients.

This stage audits factorization and block composition.
"""

from itertools import product
from collections import Counter

def poly(weights):
    c=Counter({0:1})
    for g in weights:
        d=Counter(c)
        for k,v in c.items():
            d[k+g]+=v
        c=d
    return c

def multiply(a,b):
    c=Counter()
    for i,ai in a.items():
        for j,bj in b.items():
            c[i+j]+=ai*bj
    return c

families=[
    ((), (1,2), (3,4)),
    ((1,), (1,2,4), (8,)),
    ((1,2,3), (1,1,2), (2,3)),
    ((2,5,9), (1,3,3), (4,4)),
    ((1,1,1), (2,2), (1,4)),
]

for base,G,H in families:
    Q=poly(base)
    AG=poly(G)
    AH=poly(H)

    lhs=multiply(multiply(Q,AG),AH)
    rhs=multiply(Q,multiply(AG,AH))
    direct=poly(base+G+H)

    assert lhs==rhs==direct

    # Future-block commutativity.
    assert multiply(AG,AH)==multiply(AH,AG)
    assert poly(G+H)==poly(H+G)

# Stage-129 non-Markovian histogram counterexample remains:
Q1=poly((1,1,1))
Q2=poly((2,2,2))
hist1=Counter(Q1.values())
hist2=Counter(Q2.values())
assert hist1==hist2

A=poly((1,))
R1=multiply(Q1,A)
R2=multiply(Q2,A)
assert Counter(R1.values()) != Counter(R2.values())

print("STAGE 134 — CURRENT/FUTURE CONVOLUTION FACTORIZATION")
print("="*98)
print("134A current state polynomial:")
print("  Q_cur(x)=sum_k q_k x^k")
print()
print("134B future schedule polynomial:")
print("  A_G(x)=prod_i(1+x^{g_i})=sum_s a_s x^s")
print()
print("134C exact finite-horizon evolution:")
print("  Q_final(x)=Q_cur(x) A_G(x)")
print("  q_final(k)=sum_s a_s q_{k-s}")
print()
print("134D block composition:")
print("  A_G A_H = A_{G concatenated H}")
print("  future blocks compose associatively and commute at aggregate level")
print()
print("134E state sufficiency:")
print("  full coefficient sequence q_k is sufficient for arbitrary future blocks")
print()
print("134F histogram insufficiency:")
print("  equal coefficient multisets can occupy different positions")
print("  AND polynomial multiplication distinguishes those placements")
print("  recovering the Stage-129 counterexample")
print()
print("134G exact audit:")
print("  factorization + associative block composition across multiple families: TRUE")
print()
print("PARACONSISTENT LANDING")
print("  future dynamics factor cleanly as polynomial multiplication")
print("  AND coefficient placement remains essential state information.")
print()
print("STAGE 134 RESULT : True")
