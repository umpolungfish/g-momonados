#!/usr/bin/env python3
"""
STAGE 113 — GENERAL BINARY TRAFFIC CODE / ALL-DEPTH DECODER

For m optional schedule coordinates use execution-order weights
    (1,2,4,...,2^(m-1)).

Then
    tau(b) = sum_{i=0}^{m-1} b_i 2^i
is the ordinary binary numeral of the schedule word b.

Hence:
    encode : {0,1}^m -> {0,...,2^m-1}
is bijective,
and
    b_i = floor(tau / 2^i) mod 2.

In the nested-frame realization with local multiplicities
    (2^m, 2^(m-1), ..., 2, 1)
from outermost to innermost:
    W = 2^(m+1)-1
    g1 = 2^m
    optional weights = 2^(m-1),...,1
    Delta = W-g1 = 2^m-1.

For any schedule:
    C = W + tau
    R = g1 + tau
    C-R = 2^m-1.

This file audits encode/decode for m=1..12.
"""

from itertools import product

def encode(bits):
    return sum((b<<i) for i,b in enumerate(bits))

def decode(tau,m):
    return tuple((tau>>i)&1 for i in range(m))

for m in range(1,13):
    for bits in product((0,1), repeat=m):
        t=encode(bits)
        assert 0 <= t < (1<<m)
        assert decode(t,m)==bits

    W=(1<<(m+1))-1
    g1=1<<m
    delta=(1<<m)-1
    assert W-g1==delta

    for t in range(1<<m):
        C=W+t
        R=g1+t
        assert C-R==delta

print("STAGE 113 — GENERAL BINARY TRAFFIC CODE / ALL-DEPTH DECODER")
print("="*92)
print("113A schedule code:")
print("  tau(b)=sum_i b_i 2^i")
print()
print("113B decoder:")
print("  b_i=floor(tau/2^i) mod 2")
print()
print("113C bijection:")
print("  {0,1}^m <-> {0,...,2^m-1}")
print("  executable audit m=1..12: TRUE")
print()
print("113D nested-frame realization:")
print("  local multiplicities outer->inner:")
print("    (2^m,2^(m-1),...,2,1)")
print("  W=2^(m+1)-1")
print("  g1=2^m")
print("  Delta=2^m-1")
print()
print("113E traffic law:")
print("  C=W+tau")
print("  R=g1+tau")
print("  C-R=Delta")
print()
print("113F status:")
print("  all-depth mathematical/executable theorem;")
print("  native anchor currently m=4 (Stage 110).")
print()
print("PARACONSISTENT LANDING")
print("  aggregate traffic is a complete binary schedule code")
print("  AND endpoint semantics may still collapse the entire code sector.")
print()
print("STAGE 113 RESULT : True")
