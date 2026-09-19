#!/usr/bin/env python3
"""
STAGE 94 — WEIGHTED BOOLEAN SCHEDULE QUOTIENT

Native depth-4 schedule cube from Stages 92–93.

Coordinates:
    b=(b2,b3,b4) in {0,1}^3
where bi=1 means envelope Γ_i is re-cleared.

Measured suffix-envelope traffic weights:
    g2=3, g3=2, g4=1

Weighted traffic projection:
    τ(b)=3 b2 + 2 b3 + 1 b4

Aggregate traffic:
    C=10+τ
    R= 4+τ

Generating polynomial:
    Q(x)=Π_i (1+x^{g_i})
        =(1+x^3)(1+x^2)(1+x)
        =1+x+x^2+2x^3+x^4+x^5+x^6

The coefficient [x^k]Q is exactly |τ^{-1}(k)|.
"""

from itertools import product

weights = (3,2,1)  # coordinates b2,b3,b4
cube = list(product((0,1), repeat=3))

def tau(b):
    return sum(w*x for w,x in zip(weights,b))

fibres = {}
for b in cube:
    fibres.setdefault(tau(b), []).append(b)

assert {k:len(v) for k,v in fibres.items()} == {
    0:1,1:1,2:1,3:2,4:1,5:1,6:1
}
assert set(fibres[3]) == {(1,0,0),(0,1,1)}

def hamming(a,b):
    return sum(x != y for x,y in zip(a,b))

assert hamming((1,0,0),(0,1,1)) == 3

# Positive weights imply each fibre is an antichain.
def leq(a,b):
    return all(x <= y for x,y in zip(a,b))

for vals in fibres.values():
    for i,a in enumerate(vals):
        for b in vals[i+1:]:
            assert not leq(a,b)
            assert not leq(b,a)

# Complement symmetry.
total = sum(weights)
for b in cube:
    bc = tuple(1-x for x in b)
    assert tau(bc) == total - tau(b)

print("STAGE 94 — WEIGHTED BOOLEAN SCHEDULE QUOTIENT")
print("="*82)
print("94A schedule cube:")
print("  B3 = {0,1}^3 with coordinates (b2,b3,b4)")
print("  bi=1 iff reconstructed Γ_i is re-cleared.")
print()
print("94B weighted quotient:")
print("  τ(b)=3 b2 + 2 b3 + b4")
print("  C=10+τ, R=4+τ")
print()
print("94C traffic fibres:")
for k in sorted(fibres):
    print(f"  τ={k}: {fibres[k]}")
print()
print("94D first nontrivial fibre:")
print("  τ^-1(3) = {(1,0,0),(0,1,1)}")
print("  i.e. {Γ2} and {Γ3,Γ4}.")
print("  Their Hamming distance is 3.")
print()
print("94E antichain theorem: TRUE")
print("  because all g_i>0, E⊂F implies τ(E)<τ(F).")
print("  Therefore every equal-traffic fibre is an antichain in the schedule cube.")
print()
print("94F complement symmetry: TRUE")
print("  τ(E^c)=6-τ(E)")
print("  hence Q(x)=x^6 Q(x^-1) and the fibre counts are palindromic.")
print()
print("94G projection hierarchy:")
print("  Boolean schedule cube B3")
print("    -> weighted subset-sum τ")
print("    -> aggregate traffic (10+τ,4+τ)")
print("    -> defect 6")
print("    -> endpoint T×4")
print("    -> support T")
print()
print("PARACONSISTENT LANDING")
print("  the schedule cube has full Boolean geometry")
print("  AND aggregate traffic sees only weighted subset sums.")
print("  equal-traffic fibres collapse paths")
print("  AND remain antichains of genuinely distinct schedules.")
print()
print("STAGE 94 RESULT : True")
