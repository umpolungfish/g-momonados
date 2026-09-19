#!/usr/bin/env python3
"""
STAGE 91 — AREV SCHEDULE CUBE / TRAFFIC GENERATING POLYNOMIAL

Grounded in the complete Stage-90 depth-3 schedule family.

For a depth-d nest with suffix envelopes Γ1,...,Γd and raw lane weight W,
an optional extra AREV before each remaining pop i=2,...,d defines a binary
schedule E ⊆ {2,...,d}.

Measured traffic law:
    C_E = W + Σ_{i∈E} g_i
    R_E = g_1 + Σ_{i∈E} g_i
where g_i = ||Γ_i||_1.

Therefore the bivariate schedule polynomial is
    P(u,v) = u^W v^g1 ∏_{i=2}^d (1 + (uv)^g_i).

For the native Stage-90 vector case:
    W=6, (g1,g2,g3)=(4,3,2)
so
    P(u,v)=u^6 v^4 (1+(uv)^3)(1+(uv)^2)
with monomials for exactly:
    (C,R)=(6,4),(8,6),(9,7),(11,9).
"""

from itertools import combinations

W = 6
g = {1:4, 2:3, 3:2}
measured = {(6,4), (8,6), (9,7), (11,9)}

pairs = set()
for mask in range(1 << 2):
    E = []
    for bit, i in enumerate((2,3)):
        if mask & (1 << bit):
            E.append(i)
    extra = sum(g[i] for i in E)
    pairs.add((W + extra, g[1] + extra))

assert pairs == measured

# Univariate extra-traffic polynomial coefficients for
# (1+x^2)(1+x^3) = 1 + x^2 + x^3 + x^5.
coeff = {0:1}
for gi in (g[2], g[3]):
    nxt = dict(coeff)
    for e,c in coeff.items():
        nxt[e+gi] = nxt.get(e+gi, 0) + c
    coeff = nxt

assert coeff == {0:1, 2:1, 3:1, 5:1}

print("STAGE 91 — AREV SCHEDULE CUBE / TRAFFIC GENERATING POLYNOMIAL")
print("="*84)
print("91A complete depth-3 Boolean schedule cube: TRUE")
print("  E=∅       -> C/R=6/4")
print("  E={Γ3}    -> C/R=8/6")
print("  E={Γ2}    -> C/R=9/7")
print("  E={Γ2,Γ3} -> C/R=11/9")
print()
print("91B affine schedule map:")
print("  C_E = W + Σ_{i∈E} g_i")
print("  R_E = g1 + Σ_{i∈E} g_i")
print("  with W=6 and (g1,g2,g3)=(4,3,2).")
print()
print("91C bivariate traffic polynomial:")
print("  P(u,v)=u^6 v^4 (1+(uv)^3)(1+(uv)^2)")
print("  support(P)={(6,4),(8,6),(9,7),(11,9)}")
print()
print("91D extra-traffic polynomial:")
print("  Q(x)=(1+x^3)(1+x^2)=1+x^2+x^3+x^5")
print("  every coefficient is 1 in this measured case, so each traffic total")
print("  identifies a unique AREV schedule.")
print()
print("91E defect hyperplane:")
print("  every monomial satisfies C-R=2.")
print("  Thus the whole schedule cube lies on the affine line C-R=Δ.")
print()
print("91F general theorem candidate:")
print("  for binary AREV schedules at depth d,")
print("      P(u,v)=u^W v^g1 ∏_{i=2}^d (1+(uv)^g_i)")
print("  and")
print("      Q(x)=∏_{i=2}^d (1+x^g_i)")
print("  encodes schedule multiplicities by extra traffic.")
print()
print("PARACONSISTENT LANDING")
print("  schedules are combinatorially distinct")
print("  AND can share the same endpoint and defect.")
print("  traffic is path-sensitive")
print("  AND its entire finite family is algebraically enumerable.")
print()
print("STAGE 91 RESULT : True")
