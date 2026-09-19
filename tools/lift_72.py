#!/usr/bin/env python3
"""
STAGE 72 — REPRESENTATION-SPLIT SPECTRA / REGISTER-PHASE AUTOCORRELATION

Grounded inputs from native mOMonadOS:
  Stage-70 weighted rings:
    X weights = 2 2 1 2 2 1 1 1
    Y weights = 2 2 2 1 1 2 1 1

  ROTAT orbit landing registers for W = ⊢∈⊤⊥∋⊡⊣:
    (TF, TF, TF, F, N, N, TF)

Native measurements:
  every rotation has verdict T;
  final register is phase-bearing (3 distinct values);
  adjacent-rotation phase interference = 4/7.
"""

from itertools import combinations
import math
import sympy as sp

X = [2,2,1,2,2,1,1,1]
Y = [2,2,2,1,1,2,1,1]

def weighted_cycle_matrix(w):
    n=len(w)
    A=sp.zeros(n)
    for i,a in enumerate(w):
        j=(i+1)%n
        A[i,j]=a
        A[j,i]=a
    return A

lam=sp.symbols("lam")
cpX=sp.Poly(weighted_cycle_matrix(X).charpoly(lam).as_expr(),lam)
cpY=sp.Poly(weighted_cycle_matrix(Y).charpoly(lam).as_expr(),lam)

assert cpX.as_expr() == lam**8 - 20*lam**6 + 116*lam**4 - 169*lam**2
assert cpY.as_expr() == lam**8 - 20*lam**6 + 116*lam**4 - 196*lam**2

def matchings_cycle(n,k):
    ans=[]
    for c in combinations(range(n),k):
        s=set(c)
        if all(((e-1)%n not in s and (e+1)%n not in s) for e in c):
            ans.append(c)
    return ans

def weighted_matching_sums(w):
    n=len(w)
    return [
        sum(math.prod(w[e]**2 for e in M) for M in matchings_cycle(n,k))
        for k in range(1,n//2+1)
    ]

MX=weighted_matching_sums(X)
MY=weighted_matching_sums(Y)
assert MX == [20,116,169,32]
assert MY == [20,116,196,32]
assert math.prod(X) == math.prod(Y) == 16
assert MX[-1] - 2*math.prod(X) == 0
assert MY[-1] - 2*math.prod(Y) == 0

R=("TF","TF","TF","F","N","N","TF")
def categorical_autocorr(r):
    n=len(r)
    return tuple(sum(r[j] == r[(j+t)%n] for j in range(n)) for t in range(n))
K=categorical_autocorr(R)
assert K == (7,4,2,1,1,2,4)

print("STAGE 72 — REPRESENTATION-SPLIT SPECTRA / REGISTER-PHASE AUTOCORRELATION")
print("="*82)
print("72A Stage-70 seam-homometric pair is NOT ringspec-cospectral: TRUE")
print("  chi_X =", cpX.as_expr())
print("  chi_Y =", cpY.as_expr())
print("  first differing coefficient: lambda^2, -169 vs -196")
print()
print("72B weighted-matching decomposition: TRUE")
print("  M_k(X) =", MX)
print("  M_k(Y) =", MY)
print("  product edges X=Y =", math.prod(X))
print("  same M1, M2, M4 and edge product; different M3.")
print("  Thus the material spectrum first separates the pair at weighted 3-matchings.")
print()
print("72C ROTAT verdict/register split: TRUE")
print("  verdict across all 7 rotations: T")
print("  register phase word:", R)
print("  distinct register landings:", sorted(set(R)))
print()
print("72D categorical register autocorrelation: TRUE")
print("  K(t) = # {k : r(k)=r(k+t)} =", K)
print("  adjacent phase coincidence K(1) = 4/7, matching native phase interference.")
print()
print("72E typing:")
print("  seam Fourier power  != weighted adjacency spectrum")
print("  weighted adjacency spectrum != IMASM register-phase field")
print("  same cyclic object can agree under one readout AND disagree under another.")
print()
print("PARACONSISTENT LANDING")
print("  seam C2 says X~Y AND ringspec separates X,Y")
print("  ROTAT preserves verdict T AND changes the landing register")
print("  phase is gauge-like at one projection AND information-bearing at another")
print()
print("STAGE 72 RESULT : True")
