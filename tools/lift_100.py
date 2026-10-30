#!/usr/bin/env python3
"""
STAGE 100 — PASCAL TRANSFER OPERATOR / BINOMIAL SEMIGROUP

Native anchors:
    B3 -> (1,3,3,1)
    B4 -> (1,4,6,4,1)
    B5 -> (1,5,10,10,5,1)

Define the Pascal transfer operator P on finite sequences f by

    (P f)_k = f_k + f_{k-1}

with f_{-1}=0 and f_{n+1}=0 outside the current support.

Equivalently, under generating functions

    F(x) = Σ_k f_k x^k,

P acts by multiplication with (1+x):

    F(x) -> (1+x) F(x).

Therefore

    P^n (1) = (C(n,0), ..., C(n,n)).

The transfer operators form a semigroup:

    P^a P^b = P^(a+b),

corresponding to

    (1+x)^a (1+x)^b = (1+x)^(a+b).

This file audits the operator identities exactly.
"""

from math import comb

def P(f):
    out = [0]*(len(f)+1)
    for k in range(len(out)):
        a = f[k] if k < len(f) else 0
        b = f[k-1] if k-1 >= 0 else 0
        out[k] = a+b
    return out

def Pn(f,n):
    out = list(f)
    for _ in range(n):
        out = P(out)
    return out

# Native rows
B3 = [1,3,3,1]
B4 = [1,4,6,4,1]
B5 = [1,5,10,10,5,1]

assert P(B3) == B4
assert P(B4) == B5

# General binomial row audit
for n in range(0,16):
    row = Pn([1], n)
    assert row == [comb(n,k) for k in range(n+1)]

# Semigroup law
for a in range(0,8):
    for b in range(0,8):
        assert Pn(Pn([1],a),b) == Pn([1],a+b)

# Mass doubling
for n in range(0,12):
    row = Pn([1],n)
    assert sum(row) == 2**n

# First moment / mean rank
for n in range(1,12):
    row = Pn([1],n)
    total = sum(row)
    mean_num = sum(k*v for k,v in enumerate(row))
    assert mean_num * 2 == n * total

print("STAGE 100 — PASCAL TRANSFER OPERATOR / BINOMIAL SEMIGROUP")
print("="*86)
print("100A native transfer anchors: TRUE")
print("  P(B3)=B4")
print("  P(B4)=B5")
print()
print("100B operator law:")
print("  (P f)_k = f_k + f_{k-1}")
print()
print("100C generating-function representation:")
print("  F(x) -> (1+x)F(x)")
print()
print("100D binomial orbit:")
print("  P^n(1) = (C(n,0),...,C(n,n))")
print("  exact audit n=0..15: TRUE")
print()
print("100E semigroup law:")
print("  P^a P^b = P^(a+b)")
print("  exact audit 0<=a,b<=7: TRUE")
print()
print("100F conserved/scaled quantities:")
print("  total resolved schedule count doubles each step:")
print("      Σ_k f'_k = 2 Σ_k f_k")
print("  hence Σ_k C(n,k)=2^n")
print()
print("100G rank centroid:")
print("  normalized mean traffic rank = n/2")
print("  exact first-moment audit n=1..11: TRUE")
print()
print("PARACONSISTENT LANDING")
print("  resolved schedule space doubles under cube extension")
print("  AND aggregate fibres recombine linearly by P.")
print("  Boolean geometry grows exponentially")
print("  AND rank statistics evolve under an exact linear transfer operator.")
print()
print("STAGE 100 RESULT : True")
