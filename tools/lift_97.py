#!/usr/bin/env python3
"""
STAGE 97 — ALL-DEPTH EQUAL-WEIGHT BOOLEAN-RANK THEOREM

Conditional theorem from the established equal-envelope schedule mechanics.

For depth d >= 1, put one copy of the same lane atom in each nested frame.
Then every suffix-max envelope has norm 1:

    g1=...=gd=1.

There are d-1 optional re-clear positions, so schedules are
    b in B_{d-1} = {0,1}^{d-1}.

Aggregate traffic:
    τ(b)=|b|
    C_b=d+|b|
    R_b=1+|b|
    Δ=d-1

Generating polynomial:
    Q_d(x)=(1+x)^{d-1}

Traffic-fibre cardinality:
    |τ^{-1}(k)| = C(d-1,k)

Resolved traffic code:
    C_d R1 followed, for each bit in execution order, by
      bit 0 -> R0
      bit 1 -> C1 R1

Hence the resolved code is injective, while aggregate traffic is exactly
the Boolean-rank quotient.

This file audits the formulas exhaustively for d=1..12.
"""

from itertools import product
from math import comb
from collections import Counter

def resolved_word(d, bits):
    out = [f"C{d}", "R1"]
    for b in bits:
        if b:
            out += ["C1", "R1"]
        else:
            out += ["R0"]
    return tuple(out)

for d in range(1, 13):
    bits_all = list(product((0,1), repeat=max(0,d-1)))
    words = [resolved_word(d,b) for b in bits_all]
    assert len(words) == 2**max(0,d-1)
    assert len(set(words)) == len(words)

    traffic = [(d+sum(b), 1+sum(b)) for b in bits_all]
    assert all(c-r == d-1 for c,r in traffic)

    counts = Counter(sum(b) for b in bits_all)
    assert counts == Counter({k:comb(d-1,k) for k in range(d)})

print("STAGE 97 — ALL-DEPTH EQUAL-WEIGHT BOOLEAN-RANK THEOREM")
print("="*86)
print("97A executable audit d=1..12: TRUE")
print()
print("97B schedule object:")
print("  B_{d-1}={0,1}^{d-1}")
print()
print("97C aggregate quotient:")
print("  τ(b)=|b|")
print("  C_b=d+|b|")
print("  R_b=1+|b|")
print("  Δ=d-1")
print()
print("97D fibre theorem:")
print("  |τ^{-1}(k)| = C(d-1,k)")
print("  Q_d(x)=(1+x)^(d-1)")
print()
print("97E resolved-word code:")
print("  C_d R1 ; each bit 0 contributes R0, each bit 1 contributes C1 R1")
print("  This code is injective for every audited depth.")
print()
print("97F symmetry:")
print("  S_{d-1} acts by coordinate permutation.")
print("  aggregate traffic is orbit-invariant and depends only on Boolean rank.")
print()
print("97G status:")
print("  native measurements currently land the equal-weight theorem at")
print("  B3 (Stage 95) and B4 (Stage 96).")
print("  the all-depth statement is a mathematical consequence of the")
print("  established schedule rule, not a claim of native execution at all depths.")
print()
print("PARACONSISTENT LANDING")
print("  resolved traffic preserves the full Boolean schedule word")
print("  AND aggregate traffic remembers only its Hamming weight.")
print()
print("STAGE 97 EXECUTABLE RESULT : True")
