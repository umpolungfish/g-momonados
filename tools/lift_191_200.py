#!/usr/bin/env python3
r"""
STAGES 191–200 — GLOBAL SUFFIX-ENVELOPE TRANSFORM

This batch advances beyond Stage 190 by characterizing the whole map

    Sigma_d : (P(X))^d -> Desc_d(X)
    Sigma_d(q_1,...,q_d) = (Gamma_1,...,Gamma_d)
    Gamma_i = union_{j>=i} q_j.

No new native measurement is claimed.

Main new results:
- image(Sigma_d) = all descending subset chains;
- every fibre is canonically a product of powersets;
- total fibre partition recovers 2^(d|X|);
- fixing terminal union K gives (2^d-1)^|K| ordered deposit sequences;
- a lane's deepest occurrence r contributes suffix-mass r and has 2^(r-1)
  preimages;
- exact suffix-mass generating polynomial:
      [1 + sum_{r=1}^d 2^(r-1) y^r]^|X|.
"""

from itertools import product
from collections import Counter

def subsets(U):
    U=tuple(U)
    return [
        frozenset(U[i] for i in range(len(U)) if (mask>>i)&1)
        for mask in range(1<<len(U))
    ]

def suffix_unions(qs):
    out=[None]*len(qs)
    acc=frozenset()
    for i in range(len(qs)-1,-1,-1):
        acc=acc | frozenset(qs[i])
        out[i]=acc
    return tuple(out)

def descending(chain):
    return all(chain[i] >= chain[i+1] for i in range(len(chain)-1))

def novelty(chain):
    if not chain:
        return tuple()
    return tuple(
        (chain[i]-chain[i+1]) if i+1<len(chain) else chain[i]
        for i in range(len(chain))
    )

def canonical_section(chain):
    """Disjoint novelty representative."""
    return novelty(chain)

def fibre_size(chain):
    if not chain:
        return 1
    return 2 ** sum(len(chain[i]) for i in range(1,len(chain)))

def fibre_param_to_q(chain, redundancies):
    r"""
    For i<d:
      q_i = (Gamma_i \ Gamma_{i+1}) union R_i,  R_i subseteq Gamma_{i+1}.
    q_d = Gamma_d.
    """
    d=len(chain)
    qs=[]
    for i in range(d-1):
        qs.append((chain[i]-chain[i+1]) | frozenset(redundancies[i]))
    qs.append(chain[-1])
    return tuple(qs)

def enumerate_fibre(U, chain):
    Ss=subsets(U)
    d=len(chain)
    return [
        qs for qs in product(Ss, repeat=d)
        if suffix_unions(qs)==tuple(chain)
    ]

def all_desc_chains(U,d):
    Ss=subsets(U)
    return [G for G in product(Ss, repeat=d) if descending(G)]

print("STAGE 191 — GLOBAL SUFFIX-ENVELOPE TRANSFORM")
print("="*98)
print("  Sigma_d(q_1,...,q_d)=(Gamma_1,...,Gamma_d)")
print("  Gamma_i=union_{j>=i} q_j")
print("  codomain candidate: descending chains Gamma_1 superset ... superset Gamma_d")
print("STAGE 191 RESULT : True")
print()

# ---------------------------------------------------------------------------
# 192 image theorem
# ---------------------------------------------------------------------------
for n in range(0,4):
    U=tuple(range(n))
    Ss=subsets(U)
    for d in range(1,4):
        image={suffix_unions(qs) for qs in product(Ss, repeat=d)}
        desc=set(all_desc_chains(U,d))
        assert image==desc

print("STAGE 192 — IMAGE THEOREM")
print("  Im(Sigma_d) = Desc_d(X), all descending subset chains")
print("  surjection witnessed by the novelty section:")
print("    q_i=Gamma_i\\Gamma_{i+1}, q_d=Gamma_d")
print("  exhaustive |X|<=3, d<=3 audit: TRUE")
print("STAGE 192 RESULT : True")
print()

# ---------------------------------------------------------------------------
# 193 canonical product fibre theorem
# ---------------------------------------------------------------------------
for n in range(0,4):
    U=tuple(range(n))
    for d in range(1,4):
        for G in all_desc_chains(U,d):
            if d==1:
                params=[()]
            else:
                choices=[subsets(G[i+1]) for i in range(d-1)]
                params=product(*choices)
            qs_from_params=[]
            for R in params:
                q=fibre_param_to_q(G,R)
                assert suffix_unions(q)==G
                qs_from_params.append(q)
            exact=set(enumerate_fibre(U,G))
            assert set(qs_from_params)==exact
            assert len(exact)==fibre_size(G)

print("STAGE 193 — CANONICAL PRODUCT DECOMPOSITION OF EACH FIBRE")
print("  Sigma_d^{-1}(Gamma) ~= product_{i=1}^{d-1} P(Gamma_{i+1})")
print("  via q_i=(Gamma_i\\Gamma_{i+1}) union R_i, R_i subseteq Gamma_{i+1}")
print("  q_d=Gamma_d")
print("  Stage-183 cardinality follows immediately")
print("STAGE 193 RESULT : True")
print()

# ---------------------------------------------------------------------------
# 194 global partition identity
# ---------------------------------------------------------------------------
for n in range(0,5):
    U=tuple(range(n))
    for d in range(1,5):
        lhs=sum(fibre_size(G) for G in all_desc_chains(U,d))
        rhs=2**(d*n)
        assert lhs==rhs

print("STAGE 194 — GLOBAL FIBRE-PARTITION IDENTITY")
print("  sum_{Gamma in Desc_d(X)} |Sigma_d^{-1}(Gamma)| = 2^(d|X|)")
print("  equivalently:")
print("  sum_Gamma 2^(sum_{i=2}^d |Gamma_i|) = 2^(d|X|)")
print("  exhaustive |X|<=4, d<=4 audit: TRUE")
print("STAGE 194 RESULT : True")
print()

# ---------------------------------------------------------------------------
# 195 fixed terminal union K count
# ---------------------------------------------------------------------------
def union_all(qs):
    z=frozenset()
    for q in qs:
        z |= q
    return z

for k in range(0,5):
    K=tuple(range(k))
    Ss=subsets(K)
    for d in range(1,5):
        count=sum(1 for qs in product(Ss, repeat=d) if union_all(qs)==frozenset(K))
        assert count==(2**d-1)**k

print("STAGE 195 — FIXED TERMINAL-UNION COUNT")
print("  for |K|=k:")
print("    #{(q_1,...,q_d): union_i q_i = K} = (2^d-1)^k")
print("  each lane independently chooses a nonempty subset of the d frames")
print("STAGE 195 RESULT : True")
print()

# ---------------------------------------------------------------------------
# 196 chain-refined fixed-union identity
# ---------------------------------------------------------------------------
for k in range(0,5):
    K=frozenset(range(k))
    for d in range(1,5):
        chains=[G for G in all_desc_chains(tuple(K),d) if G[0]==K]
        lhs=sum(fibre_size(G) for G in chains)
        rhs=(2**d-1)**k
        assert lhs==rhs

print("STAGE 196 — CHAIN-REFINED TERMINAL-UNION IDENTITY")
print("  sum over descending Gamma with Gamma_1=K of")
print("    2^(sum_{i=2}^d |Gamma_i|)")
print("  equals (2^d-1)^|K|")
print("  provenance chains partition the fixed-union fibre exactly")
print("STAGE 196 RESULT : True")
print()

# ---------------------------------------------------------------------------
# 197 deepest-occurrence encoding
# ---------------------------------------------------------------------------
def occurrence_pattern(mask,d):
    return tuple(i+1 for i in range(d) if (mask>>i)&1)

for d in range(1,8):
    c=Counter()
    for mask in range(1<<d):
        occ=occurrence_pattern(mask,d)
        r=max(occ) if occ else 0
        c[r]+=1
    assert c[0]==1
    for r in range(1,d+1):
        assert c[r]==2**(r-1)

print("STAGE 197 — PER-LANE DEEPEST-OCCURRENCE LAW")
print("  deepest occurrence r=0 means lane absent: multiplicity 1")
print("  for 1<=r<=d:")
print("    # occurrence patterns with deepest frame r = 2^(r-1)")
print("  because frame r is forced and frames 1..r-1 are arbitrary")
print("STAGE 197 RESULT : True")
print()

# ---------------------------------------------------------------------------
# 198 suffix mass / repeated restoration contribution
# ---------------------------------------------------------------------------
for d in range(1,7):
    for mask in range(1<<d):
        occ=occurrence_pattern(mask,d)
        r=max(occ) if occ else 0
        qs=tuple(frozenset(("x",)) if i+1 in occ else frozenset() for i in range(d))
        G=suffix_unions(qs)
        mass=sum(len(x) for x in G)
        assert mass==r

print("STAGE 198 — PER-LANE SUFFIX-MASS LAW")
print("  one lane with deepest occurrence r appears in Gamma_1,...,Gamma_r")
print("  therefore its total suffix-envelope mass is exactly r")
print("  repeated restoration traffic decomposes additively by lanes")
print("STAGE 198 RESULT : True")
print()

# ---------------------------------------------------------------------------
# 199 generating polynomial
# ---------------------------------------------------------------------------
def poly_mul(a,b):
    c=[0]*(len(a)+len(b)-1)
    for i,x in enumerate(a):
        for j,y in enumerate(b):
            c[i+j]+=x*y
    return c

def poly_pow(a,n):
    p=[1]
    for _ in range(n):
        p=poly_mul(p,a)
    return p

def mass_distribution(n,d):
    U=tuple(range(n))
    Ss=subsets(U)
    c=Counter()
    for qs in product(Ss, repeat=d):
        G=suffix_unions(qs)
        mass=sum(len(x) for x in G)
        c[mass]+=1
    return c

for n in range(0,4):
    for d in range(1,5):
        base=[0]*(d+1)
        base[0]=1
        for r in range(1,d+1):
            base[r]=2**(r-1)
        coeff=poly_pow(base,n)
        dist=mass_distribution(n,d)
        assert all(coeff[k]==dist[k] for k in range(len(coeff)))
        assert sum(coeff)==2**(d*n)

print("STAGE 199 — EXACT SUFFIX-MASS GENERATING POLYNOMIAL")
print("  for |X|=k:")
print("    M_{k,d}(y) = (1 + sum_{r=1}^d 2^(r-1) y^r)^k")
print("  coefficient [y^s] counts deposit sequences with total suffix mass s")
print("  exhaustive k<=3, d<=4 audit: TRUE")
print("STAGE 199 RESULT : True")
print()

# ---------------------------------------------------------------------------
# 200 checkpoint
# ---------------------------------------------------------------------------
print("STAGE 200 — GLOBAL PROVENANCE-TRANSFORM CHECKPOINT")
print("  Sigma_d : (P(X))^d -> Desc_d(X) is surjective")
print("  canonical section = novelty/disjoint representative")
print("  fibre over Gamma ~= product_{i<d} P(Gamma_{i+1})")
print()
print("  global counts:")
print("    total domain = 2^(d|X|)")
print("    fixed union K = (2^d-1)^|K|")
print()
print("  lane-wise encoding:")
print("    deepest occurrence r")
print("      multiplicity of histories = 2^(r-1)")
print("      suffix-mass contribution = r")
print()
print("  therefore the full traffic-mass enumerator factorizes per lane:")
print("    M_{k,d}(y)=(1+sum_r 2^(r-1)y^r)^k")
print()
print("PARACONSISTENT LANDING")
print("  suffix provenance is globally a powerset-union quotient")
print("  AND every quotient fibre has an exact product decomposition.")
print("  repeated traffic is a coarse additive shadow")
print("  AND its complete finite distribution is exactly enumerable.")
print()
print("STAGE 200 RESULT : True")
print()
print("BATCH 191–200 RESULT : True")
