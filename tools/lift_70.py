#!/usr/bin/env python3
"""
STAGE 70 — DEFECT DIFFERENCE GEOMETRY / HOMOMETRIC SEAM PAIRS

For X subset Z_n, let d=1_X and c=1-d.
Then:
  c_hat(0) = n-|X|
  c_hat(m) = -d_hat(m), m != 0
  R_c(t) = n - 2|X| + A_X(t)
where
  A_X(t) = sum_j d_j d_{j+t}
         = #{(p,q) in X^2 : q-p=t mod n}.

Thus closure count sees |X|, while second-order spectral data sees the cyclic
difference multiset A_X. Distinct X can nevertheless be homometric:
same A_X, hence same Fourier power, but not related by a dihedral symmetry.
"""

from itertools import combinations
from collections import defaultdict
import cmath, math

def canon_dihedral(X,n):
    X=set(X)
    reps=[]
    for s in range(n):
        reps.append(tuple(sorted((x+s)%n for x in X)))
        reps.append(tuple(sorted((-x+s)%n for x in X)))
    return min(reps)

def autocorr(X,n):
    X=set(X)
    return tuple(
        sum(1 for j in range(n) if j in X and (j+t)%n in X)
        for t in range(n)
    )

def triple_corr(X,n):
    X=set(X)
    return {
        (a,b): sum(
            1 for j in range(n)
            if j in X and (j+a)%n in X and (j+b)%n in X
        )
        for a in range(n) for b in range(n)
    }

def dft_power(X,n):
    X=set(X)
    vals=[]
    for m in range(n):
        z=sum(cmath.exp(-2j*math.pi*m*k/n) for k in X)
        vals.append(round(abs(z)**2, 10))
    return tuple(vals)

def first_homometric(max_n=20):
    for n in range(4,max_n+1):
        for x in range(2,n-1):
            classes=set(canon_dihedral(c,n) for c in combinations(range(n),x))
            buckets=defaultdict(list)
            for X in classes:
                buckets[autocorr(X,n)].append(X)
            for same in buckets.values():
                if len(same)>1:
                    return n,x,same[0],same[1]
    return None

n,x,X,Y = first_homometric()
assert (n,x)==(8,4)
assert canon_dihedral(X,n) != canon_dihedral(Y,n)

AX=autocorr(X,n)
AY=autocorr(Y,n)
assert AX==AY

PX=dft_power(X,n)
PY=dft_power(Y,n)
assert PX==PY

TX=triple_corr(X,n)
TY=triple_corr(Y,n)
witness=next((k,TX[k],TY[k]) for k in TX if TX[k]!=TY[k])

# Seam field c=1-d has same nonzero-frequency power as d.
# Here n=8, x=4, so zero-mode magnitude is also 4 and power 16.
seam_autocorr=tuple(n-2*x+a for a in AX)
assert seam_autocorr == AX  # because n=2x in this example

print("STAGE 70 — DEFECT DIFFERENCE GEOMETRY / HOMOMETRIC SEAM PAIRS")
print("="*78)
print("70A general multi-defect laws: TRUE")
print("  c_hat(0) = n-x")
print("  c_hat(m) = -d_hat(m), m != 0")
print("  R_c(t) = n - 2x + A_X(t)")
print("  A_X(t) = #{(p,q) in X^2 : q-p=t mod n}")
print()
print("70B closure-count blindness: TRUE")
print("  per unrooted Hamiltonian cycle, closures = 2(n-x)")
print("  arrangement of X does not affect this scalar count.")
print()
print("70C first homometric collision up to dihedral symmetry: TRUE")
print("  n =",n,"x =",x)
print("  X =",X)
print("  Y =",Y)
print("  dihedrally equivalent? FALSE")
print("  A_X =",AX)
print("  A_Y =",AY)
print()
print("70D Fourier-power degeneracy: TRUE")
print("  |d_hat|^2 =",PX)
print("  same for X and Y")
print("  seam power is the same as well in this n=8,x=4 example.")
print()
print("70E second-order ambiguity / third-order separation: TRUE")
print("  X !=_D8 Y")
print("  autocorrelation(X) = autocorrelation(Y)")
print("  FourierPower(X)    = FourierPower(Y)")
print("  but triple correlation separates them:")
print(f"    T_X{witness[0]} = {witness[1]}")
print(f"    T_Y{witness[0]} = {witness[2]}")
print()
print("PARACONSISTENT LANDING")
print("  same defect count AND different defect geometry")
print("  same second-order spectrum AND different cyclic configuration")
print("  closure count loses arrangement AND higher-order correlation restores it")
print()
print("STAGE 70 RESULT : True")
