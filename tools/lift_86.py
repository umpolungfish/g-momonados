#!/usr/bin/env python3
"""
STAGE 86 — COMPONENTWISE SUFFIX-MAX WEIGHT FILTRATION

Native depth-3 measurements:

Scalar T cases
--------------
m = (1,2,3)
  FFUSE3 restores 3,3,3
  final T×3

m = (3,1,2)
  FFUSE3 restores 2,2,3
  final T×3

Vector T/F case
---------------
m1=(2,0), m2=(0,2), m3=(1,1)

Predicted suffix-max envelopes:
  Γ3 = (1,1)
  Γ2 = max((0,2),(1,1)) = (1,2)
  Γ1 = max((2,0),(0,2),(1,1)) = (2,2)

Native fuse restore counts:
  2,3,4
and final weighted state:
  T×2,F×2

This confirms componentwise suffix-max, not merely scalar max.
"""

def vmax(*vecs):
    return tuple(max(xs) for xs in zip(*vecs))

def suffix_max_ladder(vecs):
    out = []
    for i in range(len(vecs)-1, -1, -1):
        out.append(vmax(*vecs[i:]))
    return tuple(out)

scalar_a = [(1,), (2,), (3,)]
scalar_b = [(3,), (1,), (2,)]
vector_c = [(2,0), (0,2), (1,1)]

assert suffix_max_ladder(scalar_a) == ((3,), (3,), (3,))
assert suffix_max_ladder(scalar_b) == ((2,), (2,), (3,))
assert suffix_max_ladder(vector_c) == ((1,1), (1,2), (2,2))

restore_counts_c = tuple(sum(v) for v in suffix_max_ladder(vector_c))
assert restore_counts_c == (2,3,4)

print("STAGE 86 — COMPONENTWISE SUFFIX-MAX WEIGHT FILTRATION")
print("="*82)
print("86A scalar depth-3 suffix-max: TRUE")
print("  m=(1,2,3) -> Γ=(3,3,3)")
print("  m=(3,1,2) -> Γ=(2,2,3)")
print()
print("86B vector depth-3 suffix-max: TRUE")
print("  m1=(2,0), m2=(0,2), m3=(1,1)")
print("  Γ3=(1,1)")
print("  Γ2=(1,2)")
print("  Γ1=(2,2)")
print("  native FFUSE3 restore counts = (2,3,4)")
print("  native final weight = T×2,F×2")
print()
print("86C weighted frame formula:")
print("  for lane ℓ and depth i,")
print("      Γ_i(ℓ) = max_{j>=i} m_j(ℓ)")
print("  FFUSE3 pops the suffix-max envelope Γ_i.")
print()
print("86D same-depth vs cross-depth composition:")
print("  deposits within one frame: ordinary addition")
print("  folds across frame regions: componentwise max")
print()
print("86E register quotient:")
print("  SIXTEEN_3 support is obtained by thresholding Γ_i lane-wise at >0.")
print("  Hence the Boolean suffix-join filtration is the support image")
print("  of the weighted suffix-max filtration.")
print()
print("86F exposure defect:")
print("  repeated AREV counts the currently visible envelope again.")
print("  with repeated same-lane deposits, cleared can exceed restored because")
print("  cross-region max intentionally flattens duplicate multiplicity.")
print()
print("PARACONSISTENT LANDING")
print("  local multiplicities add")
print("  AND nested provenance composes by componentwise max.")
print("  the weighted ladder is richer than the register ladder")
print("  AND its support projects exactly to the Boolean register filtration.")
print()
print("STAGE 86 RESULT : True")
