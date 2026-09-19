#!/usr/bin/env python3
"""
STAGE 137 — FREE COMMUTATIVE APPEND MONOID

Let M be the monoid of finite multisets of positive integer weights under
multiset union.

Define

    Phi(G) = A_G(x) = product_{g in G} (1+x^g).

Then

    Phi(G union H) = Phi(G) Phi(H),
    Phi(empty) = 1.

Stage 136 proves Phi is injective: A_G uniquely decodes to the sorted
weight multiset G.

Therefore the schedule-polynomial monoid

    S = { product_g (1+x^g)^{m_g} : m_g in N, finite support }

is isomorphic to the free commutative monoid on generators indexed by
positive integers.

Consequences:
- multiplication = append-multiset union;
- factor exponents m_g are unique;
- cancellation holds inside S;
- units: only 1;
- generator (1+x^g) corresponds to one append coordinate of weight g.

This is an algebraic theorem about the aggregate append representation.
Execution order is already quotiented out.
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

def poly_from_multiset(ms):
    out=Counter({0:1})
    for g,m in sorted(ms.items()):
        for _ in range(m):
            out=multiply(out,factor(g))
    return out

def poly_from_weights(ws):
    ms=Counter(ws)
    return poly_from_multiset(ms)

def degree(p):
    return max(p) if p else -1

def exact_div(num, den):
    assert den and den[degree(den)] == 1
    rem=Counter(num)
    q=Counter()
    dd=degree(den)
    while rem and degree(rem)>=dd:
        dr=degree(rem)
        coeff=rem[dr]
        shift=dr-dd
        q[shift]+=coeff
        for k,v in den.items():
            rem[k+shift]-=coeff*v
            if rem[k+shift]==0:
                del rem[k+shift]
    assert not rem
    return Counter({k:v for k,v in q.items() if v})

def decode(A):
    A=Counter(A)
    assert A[0]==1
    out=Counter()
    while not (len(A)==1 and A.get(0,0)==1):
        pos=[k for k,v in A.items() if k>0 and v]
        assert pos
        g=min(pos)
        m=A[g]
        assert m>=1
        out[g]+=m
        divisor=Counter({0:1})
        for _ in range(m):
            divisor=multiply(divisor,factor(g))
        A=exact_div(A,divisor)
        assert A[0]==1
    return out

families=[
    (),
    (1,),
    (1,2,4,8),
    (1,1,2),
    (2,2,2),
    (1,3,3,5),
    (3,6,6,10,15),
]

for G in families:
    AG=poly_from_weights(G)
    assert decode(AG)==Counter(G)

for G in families:
    for H in families:
        lhs=poly_from_weights(G+H)
        rhs=multiply(poly_from_weights(G),poly_from_weights(H))
        assert lhs==rhs
        assert decode(rhs)==Counter(G)+Counter(H)

# Cancellation inside the schedule monoid:
for G in families:
    for H in families:
        K=(1,2,2)
        lhs=multiply(poly_from_weights(G),poly_from_weights(K))
        rhs=multiply(poly_from_weights(H),poly_from_weights(K))
        assert (lhs==rhs) == (Counter(G)==Counter(H))

print("STAGE 137 — FREE COMMUTATIVE APPEND MONOID")
print("="*94)
print("137A encoding:")
print("  Phi(G)=A_G(x)=prod_g (1+x^g)^{m_g}")
print()
print("137B monoid law:")
print("  Phi(G multiset-union H)=Phi(G)Phi(H)")
print("  Phi(empty)=1")
print()
print("137C uniqueness:")
print("  Stage-136 decoder makes Phi injective")
print("  factor multiplicities m_g are unique")
print()
print("137D structure theorem:")
print("  schedule polynomials form the free commutative monoid")
print("  on generators {(1+x^g): g>=1}")
print()
print("137E consequences:")
print("  multiplication = unordered append composition")
print("  cancellation holds")
print("  only unit is 1")
print()
print("137F executable audit:")
print("  encoding, multiplication, decoding, and cancellation: TRUE")
print()
print("PARACONSISTENT LANDING")
print("  append order is quotiented out by commutativity")
print("  AND every weight multiplicity remains uniquely recoverable.")
print()
print("STAGE 137 RESULT : True")
