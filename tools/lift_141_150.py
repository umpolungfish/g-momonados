#!/usr/bin/env python3
"""
STAGES 141–150 — TRAFFIC-POLYNOMIAL ALGEBRAIC CLOSURE BATCH

This batch deliberately closes the algebraic sector rather than creating
one-file-per-corollary churn.

Status intent:
  141–150 are mathematical/executable consequences of the already landed
  schedule-polynomial/valuation results.
  They are NOT additional native IMASM measurements.
"""

from collections import Counter
from itertools import permutations
from math import factorial

# ---------------------------------------------------------------------------
# Common exact representation
# ---------------------------------------------------------------------------

def V(weights=()):
    return Counter(weights)

def clean(v):
    return Counter({g:n for g,n in v.items() if n})

def add(a,b):
    out=Counter(a)
    for g,n in b.items():
        out[g]+=n
    return clean(out)

def sub(a,b):
    out=Counter(a)
    for g,n in b.items():
        out[g]-=n
    return clean(out)

def leq(a,b):
    keys=set(a)|set(b)
    return all(a[g] <= b[g] for g in keys)

def meet(a,b):
    keys=set(a)|set(b)
    return clean(Counter({g:min(a[g],b[g]) for g in keys}))

def join(a,b):
    keys=set(a)|set(b)
    return clean(Counter({g:max(a[g],b[g]) for g in keys}))

def nonnegative(v):
    return all(n>=0 for n in v.values())

def positive_part(v):
    return clean(Counter({g:max(n,0) for g,n in v.items()}))

def negative_part(v):
    # magnitudes of negative coordinates
    return clean(Counter({g:max(-n,0) for g,n in v.items()}))

def support_tuple(v):
    return tuple(sorted((g,n) for g,n in v.items() if n))

def append_count(v):
    return sum(v.values())

def total_weight(v):
    return sum(g*n for g,n in v.items())

def multiset_words(v):
    """Number of distinct ordered words with multiplicity vector v>=0."""
    assert nonnegative(v)
    r=append_count(v)
    den=1
    for n in v.values():
        den*=factorial(n)
    return factorial(r)//den

def interval_mobius(a,b):
    """Möbius function in the schedule-divisibility lattice."""
    if not leq(a,b):
        return 0
    d=sub(b,a)
    if any(n not in (0,1) for n in d.values()):
        return 0
    return -1 if append_count(d)%2 else 1

def interval_size(a,b):
    assert leq(a,b)
    d=sub(b,a)
    ans=1
    for n in d.values():
        ans*=n+1
    return ans

def boolean_interval(a,b):
    if not leq(a,b):
        return False
    d=sub(b,a)
    return all(n in (0,1) for n in d.values())

# ---------------------------------------------------------------------------
# 141 — Grothendieck group completion
# ---------------------------------------------------------------------------

# Formal ratios A/B are represented by valuation difference nu(A)-nu(B).
# Equality of formal ratios is exactly equality of these integer vectors.
families=[
    (), (1,), (1,2,4), (1,1,2), (2,2,2),
    (1,3,3,5), (2,5,9)
]
for A in families:
    for B in families:
        z=sub(V(A),V(B))
        # canonical reconstruction into positive/negative parts
        assert sub(positive_part(z), negative_part(z)) == z
        assert set(positive_part(z)).isdisjoint(set(negative_part(z)))

print("STAGE 141 — GROTHENDIECK GROUP COMPLETION")
print("  K(S) ≅ Z^(N_{>0}) with finite support")
print("  [A/B] <-> nu(A)-nu(B)")
print("  formal multiplication becomes vector addition")
print("STAGE 141 RESULT : True")
print()

# ---------------------------------------------------------------------------
# 142 — Positive cone / constructive realizability
# ---------------------------------------------------------------------------

for A in families:
    for B in families:
        z=sub(V(B),V(A))
        realizable=nonnegative(z)
        # Exactly the schedule divisibility criterion A |_S B.
        assert realizable == leq(V(A),V(B))

print("STAGE 142 — POSITIVE CONE / CONSTRUCTIVE REALIZABILITY")
print("  a formal group element z is an actual append block iff z_g>=0 for all g")
print("  A -> B is constructively realizable iff nu(A)<=nu(B)")
print("STAGE 142 RESULT : True")
print()

# ---------------------------------------------------------------------------
# 143 — Unique reduced formal-ratio normal form
# ---------------------------------------------------------------------------

for A in families:
    for B in families:
        z=sub(V(A),V(B))
        p=positive_part(z)
        n=negative_part(z)
        assert set(p).isdisjoint(set(n))
        assert sub(p,n)==z

        # Common-factor cancellation gives same p,n.
        common=meet(V(A),V(B))
        Ared=sub(V(A),common)
        Bred=sub(V(B),common)
        assert Ared==p and Bred==n

print("STAGE 143 — UNIQUE REDUCED FORMAL-RATIO NORMAL FORM")
print("  A/B -> A_red/B_red by cancelling coordinatewise min valuations")
print("  positive and negative supports are disjoint")
print("  reduced pair is uniquely determined by the integer valuation vector")
print("STAGE 143 RESULT : True")
print()

# ---------------------------------------------------------------------------
# 144 — Lattice-ordered abelian group
# ---------------------------------------------------------------------------

ints=[
    Counter(),
    Counter({1:1}),
    Counter({1:-1}),
    Counter({1:2,3:-1}),
    Counter({2:-3,5:4}),
    Counter({1:1,2:-2,7:1}),
]
for a in ints:
    for b in ints:
        # absorption
        assert meet(a,join(a,b))==clean(a)
        assert join(a,meet(a,b))==clean(a)
        for c in ints:
            # distributivity
            assert meet(a,join(b,c)) == join(meet(a,b),meet(a,c))
            assert join(a,meet(b,c)) == meet(join(a,b),join(a,c))
            # translation invariance of order
            if leq(a,b):
                assert leq(add(a,c),add(b,c))

print("STAGE 144 — LATTICE-ORDERED ABELIAN GROUP")
print("  K(S) has coordinatewise order, meet=min, join=max")
print("  order is translation-invariant")
print("  the group lattice is distributive")
print("STAGE 144 RESULT : True")
print()

# ---------------------------------------------------------------------------
# 145 — Additive observables
# ---------------------------------------------------------------------------

def linear(v, coeff):
    return sum(coeff(g)*n for g,n in v.items())

coeffs=[
    lambda g: 1,          # append count
    lambda g: g,          # total traffic weight / degree
    lambda g: g*g,        # example higher weighted observable
    lambda g: (-1)**g,    # signed example
]
for a in ints:
    for b in ints:
        for f in coeffs:
            assert linear(add(a,b),f)==linear(a,f)+linear(b,f)

for A in families:
    v=V(A)
    assert linear(v,lambda g:1)==append_count(v)
    assert linear(v,lambda g:g)==total_weight(v)

print("STAGE 145 — ADDITIVE OBSERVABLES AS VALUATION HOMOMORPHISMS")
print("  L_c(nu)=sum_g c(g) nu_g")
print("  every such finite-support pairing is additive")
print("  append count and total weight are special cases")
print("STAGE 145 RESULT : True")
print()

# ---------------------------------------------------------------------------
# 146 — Exact current/final transition criterion
# ---------------------------------------------------------------------------

def transition(current, final):
    """Return unique append quotient valuation, or None."""
    d=sub(final,current)
    return d if nonnegative(d) else None

for A in families:
    for B in families:
        q=transition(V(A),V(B))
        if q is None:
            assert not leq(V(A),V(B))
        else:
            assert leq(V(A),V(B))
            assert add(V(A),q)==V(B)
            # uniqueness follows by cancellativity of vectors:
            assert sub(V(B),V(A))==q

print("STAGE 146 — EXACT CURRENT/FINAL TRANSITION CRITERION")
print("  unique append block K exists iff nu(final)-nu(current)>=0 coordinatewise")
print("  then nu(K)=nu(final)-nu(current)")
print("  formal ratios outside the positive cone are not executable append histories")
print("STAGE 146 RESULT : True")
print()

# ---------------------------------------------------------------------------
# 147 — Order fibre / number of execution words
# ---------------------------------------------------------------------------

quotients=[
    V(()),
    V((1,)),
    V((1,2,4,8)),
    V((1,1,2)),
    V((2,2,2)),
    V((1,3,3,5)),
]
for q in quotients:
    r=append_count(q)
    count=multiset_words(q)
    # brute-force only for small examples
    elems=list(q.elements())
    if r<=8:
        brute=len(set(permutations(elems)))
        assert brute==count

print("STAGE 147 — EXECUTION-ORDER FIBRE CARDINALITY")
print("  for quotient multiplicities m_g and r=sum_g m_g:")
print("  # distinct ordered append words = r! / prod_g m_g!")
print("  aggregate polynomial identifies all permutations of the same multiset")
print("STAGE 147 RESULT : True")
print()

# ---------------------------------------------------------------------------
# 148 — Möbius function of the append lattice
# ---------------------------------------------------------------------------

tests=[
    (V(()),V(())),
    (V(()),V((1,))),
    (V(()),V((1,2,4))),
    (V(()),V((1,1))),
    (V((1,)),V((1,2,3))),
    (V((1,)),V((1,1,2))),
]
expected=[1,-1,-1,0,1,1]
for (a,b),e in zip(tests,expected):
    assert interval_mobius(a,b)==e

print("STAGE 148 — MÖBIUS FUNCTION OF THE APPEND-DIVISIBILITY LATTICE")
print("  mu(A,B)=0 unless every quotient multiplicity is 0 or 1")
print("  on a squarefree quotient of rank r: mu(A,B)=(-1)^r")
print("STAGE 148 RESULT : True")
print()

# ---------------------------------------------------------------------------
# 149 — Boolean interval criterion
# ---------------------------------------------------------------------------

pairs=[
    (V(()),V((1,2,4))),
    (V(()),V((1,1,2))),
    (V((1,)),V((1,2,3))),
    (V((1,)),V((1,1,1,2))),
]
for a,b in pairs:
    d=sub(b,a)
    r=append_count(d)
    is_bool=boolean_interval(a,b)
    if is_bool:
        assert interval_size(a,b)==2**r
    else:
        assert interval_size(a,b)!=2**r

print("STAGE 149 — BOOLEAN INTERVAL CRITERION")
print("  [A,B] is Boolean iff the append quotient is squarefree in weight types")
print("  equivalently every quotient valuation is 0 or 1")
print("  then |[A,B]|=2^r")
print("  repeated equal weights produce product-of-chains intervals instead")
print("STAGE 149 RESULT : True")
print()

# ---------------------------------------------------------------------------
# 150 — Algebraic closure checkpoint
# ---------------------------------------------------------------------------

# Consolidated invariants:
# ordered word -> multiplicity vector -> schedule polynomial
# first map forgets order; second is bijective by Stage 136.
for q in quotients:
    # complete invariant in this algebraic sector
    canonical=support_tuple(q)
    assert clean(Counter(dict(canonical)))==q

print("STAGE 150 — TRAFFIC-POLYNOMIAL ALGEBRAIC CLOSURE CHECKPOINT")
print("  established normal form:")
print("    ordered append word")
print("      -> unordered multiplicity vector nu")
print("      <-> schedule polynomial A_G")
print()
print("  exact structure:")
print("    monoid  : N^(N_{>0}) finite support")
print("    group   : Z^(N_{>0}) finite support")
print("    product : coordinate addition")
print("    order   : coordinatewise <=")
print("    gcd/lcm : coordinatewise min/max")
print("    valid transition : nonnegative valuation difference")
print("    order fibre size : multinomial r!/prod m_g!")
print("    Boolean interval : quotient valuations in {0,1}")
print()
print("  information boundary:")
print("    multiplicity vector <-> schedule polynomial is lossless")
print("    ordered execution -> multiplicity vector forgets permutation order")
print()
print("  native qualification:")
print("    this batch adds no new native IMASM measurement;")
print("    it closes the algebra implied by the previously landed traffic laws.")
print()
print("PARACONSISTENT LANDING")
print("  aggregate append algebra has a complete canonical coordinate system")
print("  AND resolved execution order remains strictly finer information.")
print()
print("STAGE 150 RESULT : True")
print()
print("BATCH 141–150 RESULT : True")
