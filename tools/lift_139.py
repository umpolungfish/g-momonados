#!/usr/bin/env python3
"""
STAGE 139 — SCHEDULE DIVISIBILITY LATTICE / INTERNAL GCD AND LCM

For a schedule polynomial

    A_G(x)=prod_g (1+x^g)^{m_G(g)},

Stage 136 gives unique multiplicities m_G(g).

Define schedule divisibility by

    A_G |_S A_H
    iff
    m_G(g) <= m_H(g) for every g.

Then the internal meet and join are

    gcd_S(A_G,A_H)
      = prod_g (1+x^g)^{min(m_G(g),m_H(g))},

    lcm_S(A_G,A_H)
      = prod_g (1+x^g)^{max(m_G(g),m_H(g))}.

Hence the schedule-divisibility poset is the finite-support coordinatewise
lattice N^{(N_{>0})}.

Exact identities:

    gcd_S * lcm_S = A_G * A_H,

and the lattice is distributive because min/max are distributive
coordinatewise.

Important distinction from ambient polynomial gcd:

    G={1}, H={3}

have schedule gcd_S = 1, because the multisets are disjoint.

But in Z[x],

    gcd(1+x, 1+x^3) = 1+x,

since 1+x divides 1+x^3.

Thus internal append-history gcd and ambient polynomial gcd are different
operations.
"""

from collections import Counter
from math import gcd

def multiply(a,b):
    out=Counter()
    for i,ai in a.items():
        for j,bj in b.items():
            out[i+j]+=ai*bj
    return Counter({k:v for k,v in out.items() if v})

def factor(g):
    return Counter({0:1,g:1})

def poly_from_ms(ms):
    out=Counter({0:1})
    for g,m in sorted(ms.items()):
        for _ in range(m):
            out=multiply(out,factor(g))
    return out

def ms(ws):
    return Counter(ws)

def meet(a,b):
    keys=set(a)|set(b)
    return Counter({g:min(a[g],b[g]) for g in keys if min(a[g],b[g])})

def join(a,b):
    keys=set(a)|set(b)
    return Counter({g:max(a[g],b[g]) for g in keys if max(a[g],b[g])})

def leq(a,b):
    return all(a[g] <= b[g] for g in a)

families=[
    (),
    (1,),
    (3,),
    (1,2,4),
    (1,1,2),
    (2,2,2),
    (1,3,3,5),
    (2,5,9),
]

for G in families:
    for H in families:
        a,b=ms(G),ms(H)
        m=meet(a,b)
        j=join(a,b)

        # meet/join bounds
        assert leq(m,a) and leq(m,b)
        assert leq(a,j) and leq(b,j)

        # product identity
        lhs=multiply(poly_from_ms(m),poly_from_ms(j))
        rhs=multiply(poly_from_ms(a),poly_from_ms(b))
        assert lhs==rhs

        # absorption
        assert meet(a,join(a,b))==a
        assert join(a,meet(a,b))==a

# distributivity
for A in families:
    for B in families:
        for C in families:
            a,b,c=ms(A),ms(B),ms(C)
            assert meet(a,join(b,c)) == join(meet(a,b),meet(a,c))
            assert join(a,meet(b,c)) == meet(join(a,b),join(a,c))

# ambient-gcd counterexample represented by coefficient identity:
p1=poly_from_ms(ms((1,)))  # 1+x
p3=poly_from_ms(ms((3,)))  # 1+x^3
q=Counter({0:1,1:-1,2:1})
assert multiply(p1,q)==p3
assert meet(ms((1,)),ms((3,)))==Counter()

print("STAGE 139 — SCHEDULE DIVISIBILITY LATTICE / INTERNAL GCD AND LCM")
print("="*100)
print("139A valuation coordinates:")
print("  A_G <-> multiplicities m_G(g)")
print()
print("139B divisibility:")
print("  A_G |_S A_H iff m_G(g)<=m_H(g) for every g")
print()
print("139C internal gcd/meet:")
print("  gcd_S exponent at g = min(m_G(g),m_H(g))")
print()
print("139D internal lcm/join:")
print("  lcm_S exponent at g = max(m_G(g),m_H(g))")
print()
print("139E product identity:")
print("  gcd_S(A,B) * lcm_S(A,B) = A*B")
print()
print("139F lattice theorem:")
print("  schedule divisibility is a distributive lattice")
print("  isomorphic to finite-support N-valued multiplicity vectors")
print()
print("139G ambient distinction:")
print("  gcd_S(1+x,1+x^3)=1")
print("  but gcd_Z[x](1+x,1+x^3)=1+x")
print()
print("139H executable audit:")
print("  bounds, absorption, product identity, distributivity: TRUE")
print()
print("PARACONSISTENT LANDING")
print("  append-history divisibility has exact gcd/lcm internally")
print("  AND those need not match ambient polynomial gcd/lcm.")
print()
print("STAGE 139 RESULT : True")
