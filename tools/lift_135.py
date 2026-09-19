#!/usr/bin/env python3
"""
STAGE 135 — EXACT DECONVOLUTION / FUTURE-POLYNOMIAL RECOVERY

Let

    Q_final(x) = Q_cur(x) A_G(x)

with schedule polynomials in Z[x].

For every schedule polynomial produced by finite positive append weights,

    Q_cur(0)=1.

Hence Q_cur is nonzero, and Z[x] is an integral domain. Therefore if
Q_final is known and the evolution is exact, the future polynomial A_G
is uniquely determined by polynomial division:

    A_G(x) = Q_final(x) / Q_cur(x).

This recovers the complete aggregate future subset-sum multiplicity
profile a_s, though not yet the ordered append sequence.

This file performs exact integer polynomial multiplication/division
without external symbolic packages and audits multiple current/future
families.
"""

from collections import Counter

def poly_from_weights(weights):
    c=Counter({0:1})
    for g in weights:
        d=Counter(c)
        for k,v in c.items():
            d[k+g]+=v
        c=d
    return Counter({k:v for k,v in c.items() if v})

def multiply(a,b):
    c=Counter()
    for i,ai in a.items():
        for j,bj in b.items():
            c[i+j]+=ai*bj
    return Counter({k:v for k,v in c.items() if v})

def degree(p):
    return max(p) if p else -1

def exact_div(num, den):
    """
    Exact long division in Z[x].
    Requires monic denominator, which every schedule polynomial is.
    """
    assert den
    assert den[degree(den)] == 1
    rem=Counter(num)
    q=Counter()
    dd=degree(den)

    while rem and degree(rem) >= dd:
        dr=degree(rem)
        coeff=rem[dr]
        shift=dr-dd
        q[shift]+=coeff
        for k,v in den.items():
            rem[k+shift]-=coeff*v
            if rem[k+shift]==0:
                del rem[k+shift]

    assert not rem, f"non-exact division, remainder={dict(rem)}"
    return Counter({k:v for k,v in q.items() if v})

families=[
    ((), (1,2,4)),
    ((1,), (2,3)),
    ((1,2,3), (1,1,2)),
    ((1,2,4,8), (3,5)),
    ((2,5,9), (1,3,3,7)),
    ((1,1,1), (2,2,4)),
]

for current,future in families:
    Q=poly_from_weights(current)
    A=poly_from_weights(future)
    F=multiply(Q,A)

    assert Q[0]==1
    rec=exact_div(F,Q)
    assert rec==A
    assert multiply(Q,rec)==F

print("STAGE 135 — EXACT DECONVOLUTION / FUTURE-POLYNOMIAL RECOVERY")
print("="*98)
print("135A factorization:")
print("  Q_final(x)=Q_cur(x) A_G(x)")
print()
print("135B schedule-polynomial anchor:")
print("  Q_cur(0)=1")
print("  so Q_cur is nonzero and monic")
print()
print("135C exact deconvolution:")
print("  A_G(x)=Q_final(x)/Q_cur(x)")
print("  uniquely in Z[x] when the evolution is exact")
print()
print("135D recovered information:")
print("  all future subset-sum shift multiplicities a_s")
print("  are recovered exactly")
print()
print("135E qualification:")
print("  deconvolution recovers the aggregate future polynomial")
print("  but aggregate commutativity has already forgotten append order")
print()
print("135F executable audit:")
print("  repeated, binary, gapped, and collision-rich families: TRUE")
print()
print("PARACONSISTENT LANDING")
print("  aggregate evolution is forward convolution")
print("  AND exact current/final data admit unique polynomial deconvolution.")
print()
print("STAGE 135 RESULT : True")
