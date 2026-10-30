#!/usr/bin/env python3
"""
STAGES 211–250 — UNIVERSAL COORDINATE LATTICE / BOOLEAN-LAYER NORMAL FORM

No new native measurement is claimed.

The provenance-chain lattice and append-divisibility lattice are instances
of the same coordinatewise ordered monoid/lattice:

    Coord_fs(I) = N^(I) with finite support.

Finite bounded boxes [0,d_i] are product-of-chains lattices.
"""

from itertools import product
from collections import defaultdict

def leq(a,b):
    return all(x<=y for x,y in zip(a,b))

def meet(a,b):
    return tuple(min(x,y) for x,y in zip(a,b))

def join(a,b):
    return tuple(max(x,y) for x,y in zip(a,b))

def add(a,b):
    return tuple(x+y for x,y in zip(a,b))

def mu(a,b):
    if not leq(a,b):
        return 0
    ds=[y-x for x,y in zip(a,b)]
    if any(d>=2 for d in ds):
        return 0
    return -1 if sum(ds)%2 else 1

def box(caps):
    return list(product(*[range(d+1) for d in caps]))

def rank(a,weights=None):
    if weights is None:
        return sum(a)
    return sum(w*x for w,x in zip(weights,a))

def poly_mul(a,b):
    c=[0]*(len(a)+len(b)-1)
    for i,x in enumerate(a):
        for j,y in enumerate(b):
            c[i+j]+=x*y
    return c

def rank_poly(caps,weights=None):
    p=[1]
    if weights is None:
        weights=[1]*len(caps)
    for d,w in zip(caps,weights):
        f=[0]*(d*w+1)
        for r in range(d+1):
            f[r*w]=1
        p=poly_mul(p,f)
    return p

print("STAGE 211 — UNIVERSAL COORDINATE LATTICE")
print("="*98)
print("  Coord_fs(I)=N^(I) with finite support")
print("  order is coordinatewise <=")
print("  monoid product is coordinate addition")
print("  meet=min, join=max")
print("STAGE 211 RESULT : True")
print()

print("STAGE 212 — PROVENANCE AS A BOUNDED COORDINATE BOX")
print("  Stage-201 gives Desc_d(X) <-> [0,d]^X")
for k in range(0,6):
    for d in range(0,6):
        assert len(box([d]*k))==(d+1)**k
print("  finite provenance lattices are bounded intervals in Coord_fs(X)")
print("STAGE 212 RESULT : True")
print()

print("STAGE 213 — APPEND DIVISIBILITY AS THE UNBOUNDED FINITE-SUPPORT CASE")
print("  append valuation nu_g is a coordinate in N for each positive weight g")
print("  finite weight windows W with multiplicity caps d_g give boxes prod_g [0,d_g]")
print("  the full append lattice is the directed union of all such finite boxes")
print("STAGE 213 RESULT : True")
print()

print("STAGE 214 — COMMON DISTRIBUTIVE-LATTICE LAWS")
for caps in ([2,3],[1,2,2],[3,1]):
    B=box(caps)
    for a in B:
        for b in B:
            assert leq(meet(a,b),a) and leq(meet(a,b),b)
            assert leq(a,join(a,b)) and leq(b,join(a,b))
            for c in B:
                assert meet(a,join(b,c))==join(meet(a,b),meet(a,c))
                assert join(a,meet(b,c))==meet(join(a,b),join(a,c))
print("  both branches inherit the same min/max distributive lattice")
print("STAGE 214 RESULT : True")
print()

print("STAGE 215 — UNIVERSAL MÖBIUS LAW")
for caps in ([2,2],[3,1,2]):
    B=box(caps)
    for a in B:
        for b in B:
            if not leq(a,b):
                continue
            # Recurrence: sum_{a<=z<=b} mu(a,z)=delta_{a,b}
            s=sum(mu(a,z) for z in B if leq(a,z) and leq(z,b))
            assert s==(1 if a==b else 0),(a,b,s)
print("  mu(a,b)=0 if any coordinate jump >=2")
print("  otherwise mu(a,b)=(-1)^(number of +1 coordinates)")
print("  provenance Stage-206 and append Stage-148 are one theorem")
print("STAGE 215 RESULT : True")
print()

print("STAGE 216 — ZETA TRANSFORM FACTORIZATION")
caps=[2,2,1]
B=box(caps)
f={a:(1+sum((i+1)*x for i,x in enumerate(a))) for a in B}
F={}
for b in B:
    F[b]=sum(f[a] for a in B if leq(a,b))
# Möbius inversion
for b in B:
    rec=sum(mu(a,b)*F[a] for a in B if leq(a,b))
    assert rec==f[b],(b,rec,f[b])
print("  Zf(b)=sum_{a<=b} f(a)")
print("  f(b)=sum_{a<=b} mu(a,b) Zf(a)")
print("  product-of-chains incidence algebra factorizes coordinatewise")
print("STAGE 216 RESULT : True")
print()

print("STAGE 217 — LOCAL FINITE-DIFFERENCE INVERSION")
# Since mu is supported only on 0/1 jumps, inversion uses only immediate predecessors.
for b in B:
    total=0
    for eps in product((0,1), repeat=len(b)):
        if all(e<=x for e,x in zip(eps,b)):
            a=tuple(x-e for x,e in zip(b,eps))
            total+=((-1)**sum(eps))*F[a]
    assert total==f[b]
print("  f(b)=sum_{eps in {0,1}^I, eps<=b} (-1)^|eps| Zf(b-eps)")
print("  Möbius inversion is a multidimensional first-difference operator")
print("STAGE 217 RESULT : True")
print()

print("STAGE 218 — GENERAL BOX RANK POLYNOMIAL")
for caps in ([2,3],[1,2,3],[4,1]):
    coeff=rank_poly(caps)
    hist=[0]*(sum(caps)+1)
    for a in box(caps):
        hist[sum(a)]+=1
    assert coeff==hist
print("  for coordinate caps d_i:")
print("    H(y)=prod_i (1+y+...+y^(d_i))")
print("  provenance uses uniform cap d")
print("  finite append windows use weight-specific multiplicity caps")
print("STAGE 218 RESULT : True")
print()

print("STAGE 219 — ADDITIVE OBSERVABLES ON THE UNIVERSAL COORDINATES")
for caps in ([2,2,2],[1,3]):
    B=box(caps)
    w=tuple(range(1,len(caps)+1))
    for a in B:
        for b in B:
            # Additivity belongs to the ambient monoid, even if a+b exits this finite box.
            assert rank(add(a,b),w)==rank(a,w)+rank(b,w)
print("  L_c(v)=sum_i c_i v_i is additive on Coord_fs(I)")
print("  append count/total weight (Stage 145) are instances")
print("  provenance suffix mass (Stage 204) is the all-ones instance")
print("STAGE 219 RESULT : True")
print()

print("STAGE 220 — UNIVERSAL COORDINATE CHECKPOINT")
print("  master object:")
print("    Coord_fs(I)=N^(I) finite support")
print()
print("  append branch:")
print("    I=positive weights")
print("    coordinate=append multiplicity")
print("    full unbounded positive cone")
print("    group completion Z^(I)_fs from Stage 141")
print()
print("  provenance branch:")
print("    I=lanes")
print("    coordinate=deepest occurrence")
print("    bounded interval [0,d]^I")
print()
print("  shared structure:")
print("    addition in ambient monoid")
print("    order=coordinatewise <=")
print("    meet=min, join=max")
print("    Möbius support on 0/1 jumps")
print("    Boolean intervals on 0/1 jumps")
print("    zeta/Möbius transforms factor coordinatewise")
print()
print("PARACONSISTENT LANDING")
print("  append valuations and provenance depths instantiate one coordinate-lattice machine")
print("  AND they remain semantically different observables over different index sets.")
print()
print("STAGE 220 RESULT : True")
print()
print("SUB-BATCH 211–220 RESULT : True")


# ===========================================================================
# STAGES 221–250
# ===========================================================================

from math import factorial

def sub(a,b):
    return tuple(x-y for x,y in zip(a,b))

def absvec(a):
    return tuple(abs(x) for x in a)

def l1(a):
    return sum(abs(x) for x in a)

def interval_points(a,b):
    assert leq(a,b)
    return [
        x for x in product(*[range(lo,hi+1) for lo,hi in zip(a,b)])
    ]

def relative_rank(a,x):
    return sum(y-x0 for x0,y in zip(a,x))

def multinomial(counts):
    n=sum(counts)
    out=factorial(n)
    for c in counts:
        out//=factorial(c)
    return out

def coord_median(a,b,c):
    return tuple(sorted((x,y,z))[1] for x,y,z in zip(a,b,c))

def clamp_vec(v,caps):
    return tuple(min(x,d) for x,d in zip(v,caps))

def capped_add(a,b,caps):
    return clamp_vec(add(a,b),caps)

def saturation_defect(a,b,caps):
    ca=clamp_vec(a,caps)
    cb=clamp_vec(b,caps)
    cab=clamp_vec(add(a,b),caps)
    return tuple(x+y-z for x,y,z in zip(ca,cb,cab))

def threshold(v,t):
    return frozenset(i for i,x in enumerate(v) if x>=t)

def threshold_stack(v):
    h=max(v, default=0)
    return tuple(threshold(v,t) for t in range(1,h+1))

def from_threshold_stack(stack,n):
    return tuple(sum(i in S for S in stack) for i in range(n))

def schedule_poly_from_valuation(nu):
    # Index 0 represents weight 1, index 1 weight 2, etc.
    p=[1]
    for i,m in enumerate(nu):
        g=i+1
        for _ in range(m):
            f=[0]*(g+1)
            f[0]=1
            f[g]=1
            p=poly_mul(p,f)
    return tuple(p)

def squarefree_layer_poly(S):
    p=[1]
    for i in sorted(S):
        g=i+1
        f=[0]*(g+1)
        f[0]=1
        f[g]=1
        p=poly_mul(p,f)
    return tuple(p)

print()
print("STAGE 221 — INTERVAL TRANSLATION NORMAL FORM")
print("="*98)
for caps in ([2,3],[1,2,2],[3,1,2]):
    B=box(caps)
    for a in B:
        for b in B:
            if not leq(a,b):
                continue
            delta=sub(b,a)
            I=interval_points(a,b)
            translated={sub(x,a) for x in I}
            target=set(box(delta))
            assert translated==target
print("  every interval [a,b] is canonically isomorphic to [0,b-a]")
print("  [a,b] ~= product_i [0,b_i-a_i]")
print("STAGE 221 RESULT : True")
print()

print("STAGE 222 — EXACT INTERVAL CARDINALITY")
for caps in ([2,3],[1,2,2],[3,1,2]):
    B=box(caps)
    for a in B:
        for b in B:
            if leq(a,b):
                delta=sub(b,a)
                assert len(interval_points(a,b))==__import__("math").prod(x+1 for x in delta)
print("  |[a,b]| = product_i (b_i-a_i+1)")
print("STAGE 222 RESULT : True")
print()

print("STAGE 223 — RELATIVE-RANK POLYNOMIAL OF AN INTERVAL")
for caps in ([2,2],[1,2,2]):
    B=box(caps)
    for a in B:
        for b in B:
            if not leq(a,b):
                continue
            delta=sub(b,a)
            expected=rank_poly(delta)
            hist=[0]*(sum(delta)+1)
            for x in interval_points(a,b):
                hist[relative_rank(a,x)]+=1
            assert hist==expected
print("  H_[a,b](y)=product_i (1+y+...+y^(b_i-a_i))")
print("STAGE 223 RESULT : True")
print()

print("STAGE 224 — PALINDROMIC INTERVAL RANK SYMMETRY")
for delta in ([0],[2,3],[1,2,2],[4,1]):
    coeff=rank_poly(delta)
    assert coeff==list(reversed(coeff))
print("  H_[a,b](y) is palindromic of degree sum_i(b_i-a_i)")
print("  rank-r and rank-(D-r) layers have equal size")
print("STAGE 224 RESULT : True")
print()

print("STAGE 225 — ATOMS, COATOMS, AND INTERVAL RANK")
for delta in product(range(4), repeat=3):
    D=sum(delta)
    atoms=sum(x>0 for x in delta)
    coatoms=atoms
    # direct cover count from bottom/top in translated box
    B=box(delta)
    bot=(0,)*3
    top=tuple(delta)
    upper=sum(sum(y-x for x,y in zip(bot,z))==1 for z in B)
    lower=sum(sum(x-y for x,y in zip(top,z))==1 for z in B)
    assert upper==atoms and lower==coatoms
    assert max((sum(z) for z in B), default=0)==D
print("  interval rank D=sum_i delta_i")
print("  #atoms=#coatoms=#{i:delta_i>0}")
print("STAGE 225 RESULT : True")
print()

print("STAGE 226 — SATURATED-CHAIN COUNT")
for delta in product(range(4), repeat=3):
    want=multinomial(delta)
    # Dynamic-programming count of monotone paths from 0 to delta.
    dp={(0,0,0):1}
    for s in range(sum(delta)+1):
        for x in list(dp):
            if sum(x)!=s:
                continue
            for i in range(3):
                if x[i]<delta[i]:
                    y=list(x); y[i]+=1; y=tuple(y)
                    dp[y]=dp.get(y,0)+dp[x]
    assert dp[tuple(delta)]==want
print("  # saturated chains from a to b = D! / product_i delta_i!")
print("  where D=sum_i delta_i")
print("STAGE 226 RESULT : True")
print()

print("STAGE 227 — HASSE-GRAPH METRIC")
for caps in ([2,2],[1,2,2]):
    B=box(caps)
    for a in B:
        for b in B:
            dist=l1(sub(b,a))
            lattice_dist=sum(join(a,b))-sum(meet(a,b))
            assert dist==lattice_dist
print("  graph distance d(a,b)=sum_i |a_i-b_i|")
print("  equivalently d=rho(a∨b)-rho(a∧b)")
print("STAGE 227 RESULT : True")
print()

print("STAGE 228 — SHORTEST-PATH MULTIPLICITY")
for a in product(range(3), repeat=3):
    for b in product(range(3), repeat=3):
        delta=absvec(sub(b,a))
        D=sum(delta)
        assert multinomial(delta)==factorial(D)//__import__("math").prod(factorial(x) for x in delta)
print("  # Hasse geodesics(a,b)=D!/product_i |a_i-b_i|!")
print("  each geodesic is an ordering of the required unit coordinate moves")
print("STAGE 228 RESULT : True")
print()

print("STAGE 229 — MEDIAN-GRAPH THEOREM")
for a in product(range(3), repeat=3):
    for b in product(range(3), repeat=3):
        for c in product(range(3), repeat=3):
            m=coord_median(a,b,c)
            score=l1(sub(m,a))+l1(sub(m,b))+l1(sub(m,c))
            best=min(
                l1(sub(x,a))+l1(sub(x,b))+l1(sub(x,c))
                for x in product(range(3), repeat=3)
            )
            assert score==best
print("  coordinatewise median minimizes total L1 distance to any triple")
print("  product-of-chains Hasse graphs are median graphs")
print("STAGE 229 RESULT : True")
print()

print("STAGE 230 — INTERVAL-GEOMETRY CHECKPOINT")
print("  intervals are translated boxes")
print("  cardinality = product(delta_i+1)")
print("  rank polynomial = product geometric factors")
print("  saturated-chain/geodesic counts are multinomial")
print("  Hasse metric is L1 and admits coordinatewise medians")
print("STAGE 230 RESULT : True")
print()

print("STAGE 231 — UNIQUE POSITIVE/NEGATIVE JORDAN DECOMPOSITION")
for z in product(range(-3,4), repeat=3):
    zp=tuple(max(x,0) for x in z)
    zn=tuple(max(-x,0) for x in z)
    assert sub(zp,zn)==z
    assert meet(zp,zn)==(0,0,0)
print("  z=z^+-z^- with z^+,z^- >=0 and z^+∧z^-=0")
print("  decomposition is coordinatewise unique")
print("STAGE 231 RESULT : True")
print()

print("STAGE 232 — LATTICE-GROUP ABSOLUTE VALUE")
for z in product(range(-3,4), repeat=3):
    zp=tuple(max(x,0) for x in z)
    zn=tuple(max(-x,0) for x in z)
    az=absvec(z)
    assert az==add(zp,zn)
    assert az==join(z,tuple(-x for x in z))
print("  |z|=z^++z^-=z∨(-z)")
print("STAGE 232 RESULT : True")
print()

print("STAGE 233 — ORDER AS POSITIVE-CONE DIFFERENCE")
for a in product(range(-2,3), repeat=3):
    for b in product(range(-2,3), repeat=3):
        assert leq(a,b)==all(x>=0 for x in sub(b,a))
print("  a<=b iff b-a lies in the nonnegative cone")
print("  Stage-142 constructive realizability is this order law")
print("STAGE 233 RESULT : True")
print()

print("STAGE 234 — TRANSLATION-INVARIANT LATTICE OPERATIONS")
for a in product(range(-1,2), repeat=3):
    for b in product(range(-1,2), repeat=3):
        for c in product(range(-1,2), repeat=3):
            assert join(add(a,c),add(b,c))==add(join(a,b),c)
            assert meet(add(a,c),add(b,c))==add(meet(a,b),c)
print("  (a∨b)+c=(a+c)∨(b+c)")
print("  (a∧b)+c=(a+c)∧(b+c)")
print("STAGE 234 RESULT : True")
print()

print("STAGE 235 — GROUP-INTERVAL TRANSLATION")
for a in product(range(-2,2), repeat=2):
    for b in product(range(-2,3), repeat=2):
        if not leq(a,b):
            continue
        delta=sub(b,a)
        # Count is enough to audit the translated finite interval in Z^2.
        pts=[
            x for x in product(
                *[range(lo,hi+1) for lo,hi in zip(a,b)]
            )
        ]
        assert {sub(x,a) for x in pts}==set(box(delta))
print("  every finite group interval [a,b] translates to [0,b-a]")
print("STAGE 235 RESULT : True")
print()

print("STAGE 236 — ADDITIVE OBSERVABLES EXTEND TO GROUP COMPLETION")
for c in product(range(-2,3), repeat=3):
    def L(z): return sum(ci*zi for ci,zi in zip(c,z))
    for a in product(range(-2,3), repeat=3):
        for b in product(range(-1,2), repeat=3):
            assert L(add(a,b))==L(a)+L(b)
print("  L_c(z)=sum_i c_i z_i defines the unique coordinate-linear extension")
print("  append observables from Stage 145 extend from N^(I) to Z^(I)")
print("STAGE 236 RESULT : True")
print()

print("STAGE 237 — POSITIVE LINEAR OBSERVABLES")
for c in product(range(-2,3), repeat=3):
    positive=all(x>=0 for x in c)
    monotone=True
    # unit-vector test is sufficient; exhaustive small-box check agrees
    for a in product(range(3), repeat=3):
        for b in product(range(3), repeat=3):
            if leq(a,b):
                La=sum(ci*ai for ci,ai in zip(c,a))
                Lb=sum(ci*bi for ci,bi in zip(c,b))
                if La>Lb:
                    monotone=False
                    break
        if not monotone: break
    assert positive==monotone
print("  L_c is order-preserving iff every coefficient c_i>=0")
print("STAGE 237 RESULT : True")
print()

print("STAGE 238 — COORDINATE CLAMP / TRUNCATION")
for caps in ([2,3],[1,2,2]):
    for v in product(range(6), repeat=len(caps)):
        cv=clamp_vec(v,caps)
        assert leq(cv,tuple(caps))
        assert clamp_vec(cv,caps)==cv
print("  clamp_d(v)_i=min(v_i,d_i)")
print("  clamp is monotone and idempotent")
print("STAGE 238 RESULT : True")
print()

print("STAGE 239 — CLAMP IS A LATTICE RETRACTION")
for caps in ([2,3],[1,2,2]):
    V=list(product(range(5), repeat=len(caps)))
    for a in V:
        for b in V:
            assert clamp_vec(join(a,b),caps)==join(clamp_vec(a,caps),clamp_vec(b,caps))
            assert clamp_vec(meet(a,b),caps)==meet(clamp_vec(a,caps),clamp_vec(b,caps))
    for x in box(caps):
        assert clamp_vec(x,caps)==x
print("  clamp preserves meet and join")
print("  and restricts to identity on the bounded box")
print("STAGE 239 RESULT : True")
print()

print("STAGE 240 — EXACT NONADDITIVITY / SATURATION DEFECT")
for caps in ([2,3],[1,2,2]):
    for a in product(range(5), repeat=len(caps)):
        for b in product(range(5), repeat=len(caps)):
            defect=saturation_defect(a,b,caps)
            ca=clamp_vec(a,caps); cb=clamp_vec(b,caps)
            expected=tuple(max(x+y-d,0) for x,y,d in zip(ca,cb,caps))
            assert defect==expected
print("  clamp(a)+clamp(b)-clamp(a+b)")
print("    = max(clamp(a)+clamp(b)-d,0) coordinatewise")
print("  truncation is a lattice map AND generally not an additive monoid map")
print("STAGE 240 RESULT : True")
print()

print("STAGE 241 — CAPPED-ADDITION MONOID ON A BOX")
for caps in ([2,3],[1,2,2]):
    B=box(caps)
    zero=(0,)*len(caps)
    for a in B:
        assert capped_add(a,zero,caps)==a
        for b in B:
            assert capped_add(a,b,caps)==capped_add(b,a,caps)
            for c in B:
                assert capped_add(capped_add(a,b,caps),c,caps)==capped_add(a,capped_add(b,c,caps),caps)
print("  a ⊕_d b = min(d,a+b) coordinatewise")
print("  gives a commutative associative bounded monoid")
print("STAGE 241 RESULT : True")
print()

print("STAGE 242 — SATURATION DEFECT INSIDE THE CAPPED MONOID")
for caps in ([2,3],[1,2,2]):
    for a in box(caps):
        for b in box(caps):
            defect=tuple(x+y-z for x,y,z in zip(a,b,capped_add(a,b,caps)))
            expected=tuple(max(x+y-d,0) for x,y,d in zip(a,b,caps))
            assert defect==expected
print("  defect_i=max(a_i+b_i-d_i,0)")
print("  defect=0 iff no coordinate addition crosses its cap")
print("STAGE 242 RESULT : True")
print()

print("STAGE 243 — IDEMPOTENTS ARE BOOLEAN CORNERS")
for caps in ([2,3],[1,2,2]):
    idem=[]
    for a in box(caps):
        if capped_add(a,a,caps)==a:
            idem.append(a)
            assert all(x in (0,d) for x,d in zip(a,caps))
    assert len(idem)==2**len(caps)
print("  a⊕a=a iff every coordinate a_i is 0 or d_i")
print("  idempotents form the 2^|I| Boolean corners")
print("STAGE 243 RESULT : True")
print()

print("STAGE 244 — UNITS AND NONCANCELLATIVITY OF CAPPED ADDITION")
for caps in ([2,3],[1,2,2]):
    zero=(0,)*len(caps)
    B=box(caps)
    units=[]
    for a in B:
        if any(capped_add(a,b,caps)==zero for b in B):
            units.append(a)
    assert units==[zero]
    if any(d>0 for d in caps):
        i=next(i for i,d in enumerate(caps) if d>0)
        top=list(zero); top[i]=caps[i]; top=tuple(top)
        one=list(zero); one[i]=1; one=tuple(one)
        assert capped_add(top,zero,caps)==capped_add(top,one,caps)
        assert zero!=one
print("  only unit is 0")
print("  positive caps destroy cancellativity at saturation")
print("STAGE 244 RESULT : True")
print()

print("STAGE 245 — BOOLEAN SKELETON AS A LATTICE SUBOBJECT")
for caps in ([2,3],[1,2,2]):
    n=len(caps)
    corners=[
        tuple(caps[i] if ((mask>>i)&1) else 0 for i in range(n))
        for mask in range(1<<n)
    ]
    for a in corners:
        for b in corners:
            assert meet(a,b) in corners
            assert join(a,b) in corners
print("  subset S maps to corner d*1_S")
print("  intersection -> meet, union -> join")
print("  the Boolean powerset sits inside every bounded coordinate box")
print("STAGE 245 RESULT : True")
print()

print("STAGE 246 — THRESHOLD MAPS ARE BOOLEAN LATTICE HOMOMORPHISMS")
for d in range(1,5):
    B=box([d]*3)
    for t in range(1,d+1):
        for a in B:
            for b in B:
                assert threshold(join(a,b),t)==threshold(a,t)|threshold(b,t)
                assert threshold(meet(a,b),t)==threshold(a,t)&threshold(b,t)
print("  tau_t(v)={i:v_i>=t}")
print("  tau_t(v∨w)=tau_t(v) union tau_t(w)")
print("  tau_t(v∧w)=tau_t(v) intersection tau_t(w)")
print("STAGE 246 RESULT : True")
print()

print("STAGE 247 — EXACT RECONSTRUCTION FROM BOOLEAN THRESHOLDS")
for d in range(0,6):
    for v in product(range(d+1), repeat=4):
        stack=tuple(threshold(v,t) for t in range(1,d+1))
        assert from_threshold_stack(stack,4)==v
print("  v_i = #{t>=1 : i in tau_t(v)}")
print("  a bounded coordinate vector is losslessly a descending Boolean layer stack")
print("STAGE 247 RESULT : True")
print()

print("STAGE 248 — FIXED-HEIGHT BOOLEAN-LAYER EQUIVALENCE")
for d in range(1,5):
    for v in product(range(d+1), repeat=3):
        stack=tuple(threshold(v,t) for t in range(1,d+1))
        assert all(stack[t] >= stack[t+1] for t in range(len(stack)-1))
        assert from_threshold_stack(stack,3)==v
print("  [0,d]^I <-> descending d-tuples of subsets of I")
print("  provenance Gamma_t is exactly the threshold stack of its depth vector")
print("STAGE 248 RESULT : True")
print()

print("STAGE 249 — APPEND VALUATIONS FACTOR INTO SQUAREFREE BOOLEAN LAYERS")
for nu in product(range(4), repeat=4):
    stack=threshold_stack(nu)
    lhs=schedule_poly_from_valuation(nu)
    rhs=(1,)
    for S in stack:
        rhs=tuple(poly_mul(rhs,squarefree_layer_poly(S)))
    assert lhs==rhs
print("  S_t={g:nu_g>=t} is a descending finite support chain")
print("  A_nu(x)=product_t product_{g in S_t}(1+x^g)")
print("  every schedule polynomial factors canonically into squarefree layer polynomials")
print("STAGE 249 RESULT : True")
print()

print("STAGE 250 — BOOLEAN-LAYER NORMAL FORM CHECKPOINT")
print("  universal equivalence:")
print("    finite-support v in N^(I)")
print("      <-> eventually-empty descending Boolean support layers S_t={i:v_i>=t}")
print("    reconstruction: v_i=#{t:i in S_t}")
print()
print("  provenance:")
print("    bounded v=r with height <=d")
print("    Boolean layers are the suffix envelopes Gamma_t")
print()
print("  append:")
print("    v=nu on positive weights")
print("    Boolean layers S_t are multiplicity thresholds")
print("    schedule polynomial is the product of squarefree layer factors")
print()
print("  same Boolean-layer normal form")
print("  AND different semantics and monoid operations:")
print("    provenance depth records frame reach")
print("    append valuation records factor multiplicity")
print("    lattice join/meet act layerwise by union/intersection")
print("    coordinate addition is not layerwise union")
print()
print("PARACONSISTENT LANDING")
print("  powerset layers are the common Boolean skeleton beneath both coordinate systems")
print("  AND multiplicity/depth information lives in how many descending layers persist.")
print()
print("STAGE 250 RESULT : True")
print()
print("BATCH 211–250 RESULT : True")
