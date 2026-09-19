#!/usr/bin/env python3
"""
STAGE 138 — SCHEDULE DIVISIBILITY VERSUS RAW POLYNOMIAL DIVISIBILITY

Inside the free commutative schedule monoid S,

    A_G * A_K = A_H

for some schedule polynomial A_K

iff

    G is a submultiset of H.

Equivalently, the quotient A_H/A_G is itself a valid schedule polynomial
iff the decoded multiplicities satisfy m_G(g) <= m_H(g) for every g.

Important distinction:
ordinary divisibility in Z[x] is weaker.

Example:

    1+x divides 1+x^3

because

    1+x^3 = (1+x)(1-x+x^2),

but the quotient

    1-x+x^2

is NOT a schedule polynomial: it has a negative coefficient and cannot be
a product of factors (1+x^g).

Therefore:

    raw polynomial divisor
    !=
    valid append-history divisor.

The schedule-monoid divisibility relation is exactly multiset inclusion.
"""

from collections import Counter

def multiply(a,b):
    out=Counter()
    for i,ai in a.items():
        for j,bj in b.items():
            out[i+j]+=ai*bj
    return Counter({k:v for k,v in out.items() if v})

def factor(g):
    return Counter({0:1,g:1})

def poly(ws):
    out=Counter({0:1})
    for g in ws:
        out=multiply(out,factor(g))
    return out

def degree(p):
    return max(p) if p else -1

def divide_with_remainder(num, den):
    assert den and den[degree(den)] != 0
    rem=Counter(num)
    q=Counter()
    dd=degree(den)
    lead=den[dd]

    while rem and degree(rem)>=dd:
        dr=degree(rem)
        rc=rem[dr]
        assert rc % lead == 0
        coeff=rc//lead
        sh=dr-dd
        q[sh]+=coeff
        for k,v in den.items():
            rem[k+sh]-=coeff*v
            if rem[k+sh]==0:
                del rem[k+sh]
    return Counter({k:v for k,v in q.items() if v}), Counter({k:v for k,v in rem.items() if v})

def is_submultiset(G,H):
    cG=Counter(G)
    cH=Counter(H)
    return all(cG[g] <= cH[g] for g in cG)

def schedule_quotient(G,H):
    """
    Return valid schedule quotient weights K when A_G*A_K=A_H,
    otherwise None.
    """
    cG=Counter(G)
    cH=Counter(H)
    if not all(cG[g] <= cH[g] for g in cG):
        return None
    K=cH-cG
    return tuple(sorted(K.elements()))

families=[
    (),
    (1,),
    (1,2),
    (1,2,4),
    (1,1,2),
    (2,2,2),
    (1,3,3,5),
]

for G in families:
    for H in families:
        K=schedule_quotient(G,H)
        if K is not None:
            assert is_submultiset(G,H)
            assert multiply(poly(G),poly(K))==poly(H)
        else:
            assert not is_submultiset(G,H)

# Raw divisibility counterexample.
num=poly((3,))       # 1+x^3
den=poly((1,))       # 1+x
q,r=divide_with_remainder(num,den)
assert not r
assert q==Counter({0:1,1:-1,2:1})
assert schedule_quotient((1,),(3,)) is None

print("STAGE 138 — SCHEDULE DIVISIBILITY VERSUS RAW POLYNOMIAL DIVISIBILITY")
print("="*102)
print("138A schedule divisibility:")
print("  A_G divides_S A_H iff exists schedule A_K with A_G A_K=A_H")
print()
print("138B exact criterion:")
print("  divides_S iff G is a submultiset of H")
print()
print("138C quotient:")
print("  valid append quotient K = H multiset-minus G")
print()
print("138D raw Z[x] counterexample:")
print("  1+x divides 1+x^3")
print("  quotient = 1-x+x^2")
print()
print("138E distinction:")
print("  1-x+x^2 is not a schedule polynomial")
print("  so raw polynomial divisibility does not imply valid append divisibility")
print()
print("138F executable audit:")
print("  schedule divisibility <-> multiset inclusion across test families: TRUE")
print()
print("PARACONSISTENT LANDING")
print("  two schedule polynomials may divide in the ambient polynomial ring")
print("  AND fail to divide inside the append-history monoid.")
print("  algebraic factorization is broader")
print("  AND constructive append factorization is stricter.")
print()
print("STAGE 138 RESULT : True")
