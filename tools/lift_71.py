#!/usr/bin/env python3
"""
STAGE 71 — BISPECTRAL RECONSTRUCTION / FOURTH-ORDER LIFT

Let d=1_X on Z_n. Define the k-point cyclic correlation
  C_k(a1,...,a{k-1})
    = sum_j d_j d_{j+a1} ... d_{j+a{k-1}}.

Its Fourier transform is the k-spectral product
  \hat C_k(m1,...,m{k-1})
    = D(m1)...D(m{k-1}) D(-sum mi)
(up to the chosen DFT sign convention).

Stage 70 showed C_2 can be homometric.
Stage 71 tests C_3 and then exhibits an exact C_3 failure at n=36,
separated by C_4.
"""

from itertools import combinations, product

def mask_of(X):
    return sum(1 << x for x in X)

def rotate_mask(mask,n,s):
    if s == 0:
        return mask
    full=(1<<n)-1
    return ((mask<<s)|(mask>>(n-s))) & full

def reflect_mask(mask,n):
    r=0
    for i in range(n):
        if (mask>>i)&1:
            r |= 1 << ((-i)%n)
    return r

def canon_dihedral_mask(mask,n):
    vals=[]
    refl=reflect_mask(mask,n)
    for s in range(n):
        vals.append(rotate_mask(mask,n,s))
        vals.append(rotate_mask(refl,n,s))
    return min(vals)

def rots_for_corr(mask,n):
    full=(1<<n)-1
    out=[]
    for a in range(n):
        # j+a in X iff j in X-a
        out.append(((mask>>a)|(mask<<(n-a))) & full if a else mask)
    return out

def triple_signature(mask,n):
    rr=rots_for_corr(mask,n)
    return tuple((mask & rr[a] & rr[b]).bit_count()
                 for a in range(n) for b in range(n))

def first_fourth_witness(m1,m2,n):
    r1=rots_for_corr(m1,n)
    r2=rots_for_corr(m2,n)
    for a,b,c in product(range(n), repeat=3):
        x=(m1 & r1[a] & r1[b] & r1[c]).bit_count()
        y=(m2 & r2[a] & r2[b] & r2[c]).bit_count()
        if x != y:
            return (a,b,c),x,y
    return None

# Exhaustive small-carrier audit: one representative per D_n orbit.
audit=[]
for n in range(3,19):
    seen={}
    classes=0
    collision=False
    for mask in range(1,(1<<n)-1):
        if canon_dihedral_mask(mask,n) != mask:
            continue
        classes += 1
        sig=triple_signature(mask,n)
        if sig in seen and seen[sig] != mask:
            collision=True
            break
        seen[sig]=mask
    audit.append((n,classes,collision))
assert not any(c for _,_,c in audit)

# Explicit n=36 same-3-deck pair from the p=2,q=3,r=2,d=3 cyclic construction.
n=36
E=(0,1,3,4,12,15,19,22,24,27)
F=(0,3,4,7,12,15,22,24,25,27)
mE=mask_of(E)
mF=mask_of(F)

assert canon_dihedral_mask(mE,n) != canon_dihedral_mask(mF,n)
assert triple_signature(mE,n) == triple_signature(mF,n)

w=first_fourth_witness(mE,mF,n)
assert w == ((1,3,4),1,0)

print("STAGE 71 — BISPECTRAL RECONSTRUCTION / FOURTH-ORDER LIFT")
print("="*78)
print("71A k-point correlation tower: TRUE")
print("  C_k(a1,...,a{k-1}) = sum_j d_j d_{j+a1}...d_{j+a{k-1}}")
print("  C_2 = autocorrelation / power-spectrum face")
print("  C_3 = triple correlation / bispectral face")
print()
print("71B exhaustive C3 audit modulo D_n, nontrivial subsets n=3..18: TRUE")
for n,classes,collision in audit:
    print(f"  n={n:2d}  D_n classes={classes:5d}  C3 collision={collision}")
print()
print("71C explicit third-order failure: TRUE")
print("  n = 36")
print("  E =",E)
print("  F =",F)
print("  E !=_D36 F")
print("  C3(E) = C3(F)")
print()
print("71D fourth-order separation: TRUE")
print("  witness shifts (1,3,4):")
print("    C4_E(1,3,4) = 1")
print("    C4_F(1,3,4) = 0")
print()
print("71E hierarchy:")
print("  C2 can lose geometry (Stage 70 homometry)")
print("  C3 restores phase on many carriers but is not universal")
print("  C4 separates this exact C3-degenerate pair")
print()
print("PARACONSISTENT LANDING")
print("  third order reconstructs every audited class through n=18")
print("  AND third order fails on an explicit n=36 pair")
print("  same bispectral data AND different dihedral geometry")
print("  fourth-order lift restores the distinction for this pair")
print()
print("STAGE 71 RESULT : True")
