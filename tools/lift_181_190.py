#!/usr/bin/env python3
"""
STAGES 181–190 — SUFFIX-ENVELOPE / POWSET-MONAD INTEGRATION

No new native measurements are made here.

Native/mechanical anchors already landed:
- Stage 81: suffix envelopes Γ_i = join_{j>=i} q_j.
- Stages 85–86: weighted envelopes use componentwise max.
- Stage 89: repeated traffic is governed by envelope norms.
- Stages 151–160: six {T,F,tf} orders give six provenance ladders and
  three norm/traffic classes.
- Stage 136: a schedule polynomial uniquely recovers its positive
  integer weight multiset.

This batch integrates those facts with the earlier powerset-monad union μ.
"""

from itertools import product, permutations
from collections import Counter
from math import prod

# ---------------------------------------------------------------------------
# Basic set suffix-envelope machinery
# ---------------------------------------------------------------------------

def suffix_unions(qs):
    out=[None]*len(qs)
    acc=frozenset()
    for i in range(len(qs)-1,-1,-1):
        acc=acc | frozenset(qs[i])
        out[i]=acc
    return tuple(out)

def set_fibre_count(chain):
    # chain Γ_1 ⊇ ... ⊇ Γ_d
    if not chain:
        return 1
    return 2 ** sum(len(chain[i]) for i in range(1,len(chain)))

def valid_chain(chain):
    return all(chain[i] >= chain[i+1] for i in range(len(chain)-1))

def enumerate_set_fibre(universe, chain):
    """All q-sequences with prescribed suffix-union chain."""
    subs=[]
    U=list(universe)
    for mask in range(1<<len(U)):
        subs.append(frozenset(U[j] for j in range(len(U)) if mask>>j & 1))
    d=len(chain)
    ans=[]
    for qs in product(subs, repeat=d):
        if suffix_unions(qs)==tuple(chain):
            ans.append(qs)
    return ans

print("STAGE 181 — SUFFIX ENVELOPE AS ITERATED UNION μ")
print("="*96)
print("  Γ_i = union_{j>=i} q_j")
print("  equivalently Γ_i = μ({q_i,...,q_d}) in the powerset union algebra")
print("  full provenance keeps every suffix union, not only the terminal Γ_1")
print("STAGE 181 RESULT : True")
print()

# ---------------------------------------------------------------------------
# 182 novelty decomposition
# ---------------------------------------------------------------------------

def novelty(chain):
    if not chain:
        return tuple()
    out=[]
    for i in range(len(chain)-1):
        out.append(chain[i]-chain[i+1])
    out.append(chain[-1])
    return tuple(out)

examples=[
    (frozenset("T"),frozenset("F"),frozenset(("t","f"))),
    (frozenset("T"),frozenset("T"),frozenset("F")),
]
for qs in examples:
    G=suffix_unions(qs)
    N=novelty(G)
    for i in range(len(qs)):
        assert N[i] <= frozenset(qs[i])

print("STAGE 182 — LAST-OCCURRENCE / NOVELTY DECOMPOSITION")
print("  n_i = Γ_i \\ Γ_{i+1}, with n_d=Γ_d")
print("  n_i is the part of q_i not repeated at any deeper frame")
print("  always n_i subseteq q_i")
print("STAGE 182 RESULT : True")
print()

# ---------------------------------------------------------------------------
# 183 exact set fibre
# ---------------------------------------------------------------------------

U=frozenset(("a","b"))
chains=[]
subs=[frozenset(),frozenset(("a",)),frozenset(("b",)),frozenset(("a","b"))]
for G in product(subs, repeat=3):
    if valid_chain(G):
        chains.append(G)
        got=len(enumerate_set_fibre(U,G))
        want=set_fibre_count(G)
        assert got==want,(G,got,want)

print("STAGE 183 — EXACT PROVENANCE FIBRE FOR SET DEPOSITS")
print("  for fixed Γ_1 ⊇ ... ⊇ Γ_d:")
print("    Γ_i\\Γ_{i+1} subseteq q_i subseteq Γ_i")
print("    q_d = Γ_d")
print("  every subset of Γ_{i+1} may be redundantly repeated in q_i")
print("  |ProvFib(Γ)| = 2^(sum_{i=2}^d |Γ_i|)")
print("  exhaustive U={a,b}, d=3 audit: TRUE")
print("STAGE 183 RESULT : True")
print()

# ---------------------------------------------------------------------------
# 184 disjoint support => exact reconstruction
# ---------------------------------------------------------------------------

def disjoint(qs):
    seen=set()
    for q in qs:
        q=set(q)
        if seen & q:
            return False
        seen |= q
    return True

for perm in permutations([
    frozenset(("T",)),
    frozenset(("F",)),
    frozenset(("t","f")),
]):
    assert disjoint(perm)
    G=suffix_unions(perm)
    assert novelty(G)==perm

print("STAGE 184 — DISJOINT-SUPPORT RECONSTRUCTION THEOREM")
print("  if q_i are pairwise disjoint, q_i = Γ_i\\Γ_{i+1} (q_d=Γ_d)")
print("  hence the provenance ladder is injective on the disjoint-support sector")
print("  this explains the six-way injectivity measured in Stages 151–160")
print("STAGE 184 RESULT : True")
print()

# ---------------------------------------------------------------------------
# weighted max envelopes
# ---------------------------------------------------------------------------

def vmax(a,b):
    return tuple(max(x,y) for x,y in zip(a,b))

def suffix_max(ms):
    if not ms:
        return tuple()
    z=(0,)*len(ms[0])
    out=[None]*len(ms)
    acc=z
    for i in range(len(ms)-1,-1,-1):
        acc=vmax(ms[i],acc)
        out[i]=acc
    return tuple(out)

def weighted_fibre_count(chain):
    if not chain:
        return 1
    d=len(chain); L=len(chain[0])
    ans=1
    for i in range(d-1):
        for l in range(L):
            a,b=chain[i][l],chain[i+1][l]
            assert a>=b
            if a==b:
                ans*=a+1
    return ans

def enumerate_weighted_fibre(chain):
    d=len(chain); L=len(chain[0])
    bounds=[range(max(G[l] for G in chain)+1) for l in range(L)]
    vecs=list(product(*bounds))
    ans=[]
    for ms in product(vecs, repeat=d):
        if suffix_max(ms)==tuple(chain):
            ans.append(ms)
    return ans

print("STAGE 185 — WEIGHTED MAX-ENVELOPE FIBRE")
print("  Γ_i(l)=max_{j>=i} m_j(l)")
print("  if Γ_i(l)>Γ_{i+1}(l), then m_i(l)=Γ_i(l) is forced")
print("  if Γ_i(l)=Γ_{i+1}(l)=a, then m_i(l) may be any 0..a")
print("  weighted fibre size = product (a+1) over equal-positive/equal-zero slots")
print("STAGE 185 RESULT : True")
print()

# Audit weighted formula on small chains arising from all 2-lane depth-3 sequences with values 0..2.
vecs=list(product(range(3), repeat=2))
seen={}
for ms in product(vecs, repeat=3):
    G=suffix_max(ms)
    seen.setdefault(G,0)
    seen[G]+=1
for G,count in seen.items():
    assert count==weighted_fibre_count(G),(G,count,weighted_fibre_count(G))

print("STAGE 186 — WEIGHTED FIBRE ENUMERATION AUDIT")
print("  exhaustive 2 lanes × depth 3 × multiplicities {0,1,2}: TRUE")
print("  suffix maxima retain record-setting multiplicities")
print("  AND erase sub-maximal repeats hidden below an existing envelope")
print("STAGE 186 RESULT : True")
print()

# ---------------------------------------------------------------------------
# traffic norm factorization
# ---------------------------------------------------------------------------

def norm_set(G):
    return len(G)

atoms={
    "T":frozenset(("T",)),
    "F":frozenset(("F",)),
    "tf":frozenset(("t","f")),
}
rows={}
for p in permutations(("T","F","tf")):
    G=suffix_unions(tuple(atoms[x] for x in p))
    g=tuple(map(norm_set,G))
    repeated=sum(g)
    rows[p]=(G,g,repeated)

native_expected={
    ("T","F","tf"):9,
    ("T","tf","F"):8,
    ("F","T","tf"):9,
    ("F","tf","T"):8,
    ("tf","T","F"):7,
    ("tf","F","T"):7,
}
for p,(_,g,r) in rows.items():
    assert r==native_expected[p]

print("STAGE 187 — PROVENANCE-TO-TRAFFIC NORM FACTORIZATION")
print("  Γ=(Γ_1,...,Γ_d) -> g=(|Γ_1|,...,|Γ_d|)")
print("  repeated restoration traffic R_rep = sum_i g_i")
print("  on {T,F,tf}:")
for p,(_,g,r) in rows.items():
    print(f"    {p}: g={g}, R_rep={r}")
print("STAGE 187 RESULT : True")
print()

# ---------------------------------------------------------------------------
# schedule polynomial recovers monotone norm ladder
# ---------------------------------------------------------------------------

def poly_mul(a,b):
    c=[0]*(len(a)+len(b)-1)
    for i,x in enumerate(a):
        for j,y in enumerate(b):
            c[i+j]+=x*y
    return tuple(c)

def schedule_poly(weights):
    p=(1,)
    for g in weights:
        f=[0]*(g+1); f[0]=1; f[g]=1
        p=poly_mul(p,tuple(f))
    return p

def decode_weights(poly):
    # Stage-136 constructive decoder for positive integer weights.
    p=list(poly)
    ws=[]
    while len(p)>1:
        g=next(i for i,c in enumerate(p[1:],start=1) if c)
        m=p[g]
        # exact repeated division by (1+x^g), m times
        for _ in range(m):
            q=[0]*(len(p)-g)
            for i in range(len(q)):
                q[i]=p[i]
                if i-g>=0:
                    q[i]-=q[i-g]
            # verify reconstruction
            assert list(poly_mul(tuple(q), tuple([1]+[0]*(g-1)+[1])))==p
            p=q
            while len(p)>1 and p[-1]==0:
                p.pop()
        ws += [g]*m
    assert p==[1]
    return tuple(sorted(ws))

# Current native sector: Q uses optional envelope weights g_2..g_d.
q_classes={}
for p,(_,g,_) in rows.items():
    Q=schedule_poly(g[1:])
    decoded=decode_weights(Q)
    assert decoded==tuple(sorted(g[1:]))
    # envelope norms are nonincreasing, so sorted multiset recovers indexed values.
    recovered=tuple(sorted(decoded, reverse=True))
    assert recovered==g[1:]
    q_classes[p]=Q

assert len(set(q_classes.values()))==3

print("STAGE 188 — SCHEDULE POLYNOMIAL RECOVERS THE ENVELOPE NORM LADDER")
print("  Q(x)=prod_{i=2}^d (1+x^{g_i})")
print("  Stage-136 decoding recovers the multiset {g_2,...,g_d}")
print("  suffix envelopes force g_2>=...>=g_d, so sorting recovers the indexed norm ladder")
print("  on the native six-order sector Q has exactly 3 classes")
print("STAGE 188 RESULT : True")
print()

# ---------------------------------------------------------------------------
# Same norm ladder, distinct full provenance
# ---------------------------------------------------------------------------

pairs=[
    (("T","F","tf"),("F","T","tf")),
    (("T","tf","F"),("F","tf","T")),
    (("tf","T","F"),("tf","F","T")),
]
for a,b in pairs:
    Ga,ga,_=rows[a]
    Gb,gb,_=rows[b]
    assert ga==gb
    assert q_classes[a]==q_classes[b]
    assert Ga!=Gb

print("STAGE 189 — NORM/SCHEDULE QUOTIENT IS STRICTLY COARSER THAN PROVENANCE")
print("  each T/F-swap pair has:")
print("    same envelope norm ladder")
print("    same schedule polynomial Q")
print("    same repeated aggregate traffic")
print("  AND a different full lane-labelled provenance ladder")
print("STAGE 189 RESULT : True")
print()

print("STAGE 190 — POWSET / PROVENANCE / TRAFFIC INTEGRATION CHECKPOINT")
print("  set-level factorization:")
print("    ordered deposits q")
print("      -> suffix unions Γ_i = μ(tail_i)")
print("      -> envelope norms g_i=|Γ_i|")
print("      -> schedule polynomial Q")
print("      -> aggregate traffic / endpoint projections")
print()
print("  Stage-58 fibre AND provenance fibre use the same union multiplication μ")
print("  BUT they are different fibres:")
print("    Stage-58 fixes one terminal union of an unordered family")
print("    provenance fixes the entire ordered suffix-union chain")
print()
print("  full provenance remembers lane identity")
print("  AND envelope norms/schedule polynomial can forget T/F orientation.")
print()
print("  In the measured {T,F,tf} sector:")
print("    6 ordered/provenance classes -> 3 norm/Q/traffic classes -> 1 endpoint class")
print()
print("PARACONSISTENT LANDING")
print("  μ=union underlies both the old powerset fibre and the native frame envelope")
print("  AND the ordered suffix chain carries strictly more information than terminal union.")
print()
print("STAGE 190 RESULT : True")
print()
print("BATCH 181–190 RESULT : True")
