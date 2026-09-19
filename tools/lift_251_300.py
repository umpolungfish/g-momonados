#!/usr/bin/env python3
"""
STAGES 251–300 — BOOLEAN CARRY ALGEBRA

No new native-machine measurement is claimed.

Let v,w in N^I have threshold stacks

    S_t(v) = {i : v_i >= t},  t>=1,

with S_0(v)=I and S_t(v)=empty above the height of v.

The new operation is Boolean convolution:

    (S ⋆ T)_t = union_{a+b=t} (S_a intersect T_b).

The central theorem is:

    S(v+w) = S(v) ⋆ S(w).

Thus coordinate addition is represented exactly inside the powerset lattice
by unions/intersections plus vertical layer carry.
"""

from itertools import product
from math import factorial, ceil

def add(a,b): return tuple(x+y for x,y in zip(a,b))
def sub(a,b): return tuple(x-y for x,y in zip(a,b))
def meet(a,b): return tuple(min(x,y) for x,y in zip(a,b))
def join(a,b): return tuple(max(x,y) for x,y in zip(a,b))
def leq(a,b): return all(x<=y for x,y in zip(a,b))
def l1(a): return sum(abs(x) for x in a)

def encode(v):
    h=max(v, default=0)
    return tuple(
        frozenset(i for i,x in enumerate(v) if x>=t)
        for t in range(1,h+1)
    )

def trim(S):
    S=list(S)
    while S and not S[-1]:
        S.pop()
    return tuple(S)

def decode(S,n):
    return tuple(sum(i in A for A in S) for i in range(n))

def layer(S,t,I):
    if t==0:
        return I
    if 1 <= t <= len(S):
        return S[t-1]
    return frozenset()

def valid_stack(S):
    return all(S[i] >= S[i+1] for i in range(len(S)-1))

def conv(S,T,n):
    I=frozenset(range(n))
    H=len(S)+len(T)
    U=[]
    for t in range(1,H+1):
        A=frozenset()
        for a in range(t+1):
            A |= layer(S,a,I) & layer(T,t-a,I)
        U.append(A)
    U=trim(U)
    assert valid_stack(U)
    return U

def stack_join(S,T):
    H=max(len(S),len(T))
    out=[]
    for t in range(1,H+1):
        a=S[t-1] if t<=len(S) else frozenset()
        b=T[t-1] if t<=len(T) else frozenset()
        out.append(a|b)
    return trim(out)

def stack_meet(S,T):
    H=max(len(S),len(T))
    out=[]
    for t in range(1,H+1):
        a=S[t-1] if t<=len(S) else frozenset()
        b=T[t-1] if t<=len(T) else frozenset()
        out.append(a&b)
    return trim(out)

def stack_leq(S,T):
    H=max(len(S),len(T))
    for t in range(1,H+1):
        a=S[t-1] if t<=len(S) else frozenset()
        b=T[t-1] if t<=len(T) else frozenset()
        if not a <= b:
            return False
    return True

def weak_compositions(total,k):
    if k==1:
        yield (total,)
        return
    for a in range(total+1):
        for rest in weak_compositions(total-a,k-1):
            yield (a,)+rest

def conv_many_direct(stacks,n):
    I=frozenset(range(n))
    H=sum(len(S) for S in stacks)
    out=[]
    for t in range(1,H+1):
        A=frozenset()
        for alpha in weak_compositions(t,len(stacks)):
            part=I
            for S,a in zip(stacks,alpha):
                part = part & layer(S,a,I)
            A |= part
        out.append(A)
    return trim(out)

def scalar(nv,k):
    return tuple(k*x for x in nv)

def clamp(v,d):
    return tuple(min(x,d) for x in v)

def indicator(S,n):
    return tuple(1 if i in S else 0 for i in range(n))

def majority3(A,B,C):
    return (A&B)|(A&C)|(B&C)

def stack_mass(S):
    return sum(len(A) for A in S)

def dot(a,b):
    return sum(x*y for x,y in zip(a,b))

# ---------------------------------------------------------------------------
print("STAGE 251 — BOOLEAN-LAYER ADDITION PROBLEM")
print("="*100)
print("  threshold encoding S_t(v)={i:v_i>=t}")
print("  lattice join/meet are layerwise union/intersection")
print("  coordinate addition requires a new vertical carry law")
print("STAGE 251 RESULT : True")
print()

print("STAGE 252 — EXACT BOOLEAN CONVOLUTION FORMULA")
for n in range(0,5):
    for v in product(range(4), repeat=n):
        Sv=encode(v)
        for w in product(range(4), repeat=n):
            Sw=encode(w)
            U=conv(Sv,Sw,n)
            assert decode(U,n)==add(v,w)
            assert U==encode(add(v,w))
print("  (S⋆T)_t = union_{a+b=t} (S_a intersect T_b), with S_0=T_0=I")
print("  encode(v+w)=encode(v)⋆encode(w)")
print("  exhaustive n<=4, coordinates 0..3: TRUE")
print("STAGE 252 RESULT : True")
print()

print("STAGE 253 — COMMUTATIVITY OF BOOLEAN CONVOLUTION")
for n in range(0,4):
    for v in product(range(3), repeat=n):
        for w in product(range(3), repeat=n):
            assert conv(encode(v),encode(w),n)==conv(encode(w),encode(v),n)
print("  S⋆T=T⋆S")
print("STAGE 253 RESULT : True")
print()

print("STAGE 254 — ASSOCIATIVITY OF BOOLEAN CONVOLUTION")
for n in range(0,4):
    vals=list(product(range(3), repeat=n))
    for v in vals:
        for w in vals:
            for z in vals:
                S,T,U=encode(v),encode(w),encode(z)
                assert conv(conv(S,T,n),U,n)==conv(S,conv(T,U,n),n)
print("  (S⋆T)⋆U=S⋆(T⋆U)")
print("  inherited exactly from associative coordinate addition")
print("STAGE 254 RESULT : True")
print()

print("STAGE 255 — IDENTITY STACK")
for n in range(0,5):
    E=tuple()
    for v in product(range(4), repeat=n):
        S=encode(v)
        assert conv(S,E,n)==S
        assert conv(E,S,n)==S
print("  empty stack encodes the zero vector and is the convolution identity")
print("STAGE 255 RESULT : True")
print()

print("STAGE 256 — SUPPORT LAYER HAS NO CARRY")
for n in range(0,5):
    for v in product(range(4), repeat=n):
        for w in product(range(4), repeat=n):
            U=encode(add(v,w))
            top = U[0] if U else frozenset()
            Sv=encode(v); Sw=encode(w)
            s1=Sv[0] if Sv else frozenset()
            t1=Sw[0] if Sw else frozenset()
            assert top==s1|t1
print("  U_1=S_1 union T_1")
print("  ordinary support is just Boolean union")
print("STAGE 256 RESULT : True")
print()

print("STAGE 257 — FIRST CARRY LAYER")
for n in range(0,5):
    for v in product(range(3), repeat=n):
        for w in product(range(3), repeat=n):
            S,T=encode(v),encode(w)
            I=frozenset(range(n))
            U=encode(add(v,w))
            u2=layer(U,2,I)
            rhs=layer(S,2,I)|layer(T,2,I)|(layer(S,1,I)&layer(T,1,I))
            assert u2==rhs
print("  U_2 = S_2 union T_2 union (S_1 intersect T_1)")
print("  overlap of first layers carries one level upward")
print("STAGE 257 RESULT : True")
print()

print("STAGE 258 — GENERAL CARRY DECOMPOSITION")
for n in range(0,4):
    for v in product(range(4), repeat=n):
        for w in product(range(4), repeat=n):
            S,T=encode(v),encode(w)
            I=frozenset(range(n))
            U=encode(add(v,w))
            for t in range(1,len(U)+1):
                rhs=layer(S,t,I)|layer(T,t,I)
                for a in range(1,t):
                    rhs |= layer(S,a,I)&layer(T,t-a,I)
                assert layer(U,t,I)==rhs
print("  U_t=S_t union T_t union union_{1<=a<t}(S_a intersect T_{t-a})")
print("  higher layers are inherited depth plus all carry intersections")
print("STAGE 258 RESULT : True")
print()

print("STAGE 259 — HEIGHT OF A SUM")
for n in range(0,5):
    for v in product(range(4), repeat=n):
        for w in product(range(4), repeat=n):
            hv=max(v,default=0); hw=max(w,default=0)
            hu=max(add(v,w),default=0)
            assert hu<=hv+hw
            assert len(encode(add(v,w)))==hu
print("  h(v+w)=max_i(v_i+w_i) <= h(v)+h(w)")
print("  equality need not hold because the maxima may occur on different coordinates")
print("STAGE 259 RESULT : True")
print()

print("STAGE 260 — BOOLEAN CONVOLUTION CHECKPOINT")
print("  threshold encoding transports coordinate addition to ⋆")
print("  support uses union; every higher layer adds explicit intersection carries")
print("  descending Boolean stacks now carry the full additive monoid structure")
print("STAGE 260 RESULT : True")
print()

# ---------------------------------------------------------------------------
print("STAGE 261 — K-ARY BOOLEAN CONVOLUTION")
for n in range(0,4):
    vals=list(product(range(3), repeat=n))
    for a in vals[:min(len(vals),8)]:
        for b in vals[:min(len(vals),8)]:
            for c in vals[:min(len(vals),8)]:
                stacks=[encode(a),encode(b),encode(c)]
                direct=conv_many_direct(stacks,n)
                iterated=conv(conv(stacks[0],stacks[1],n),stacks[2],n)
                assert direct==iterated==encode(add(add(a,b),c))
print("  U_t = union_{a_1+...+a_k=t} intersection_j S^j_{a_j}")
print("  exact multi-input carry formula")
print("STAGE 261 RESULT : True")
print()

print("STAGE 262 — PARENTHESIZATION INDEPENDENCE")
for n in range(0,4):
    vals=list(product(range(3), repeat=n))
    for a in vals[:8]:
        for b in vals[:8]:
            for c in vals[:8]:
                A,B,C=encode(a),encode(b),encode(c)
                assert conv(conv(A,B,n),C,n)==conv(A,conv(B,C,n),n)
print("  k-ary carry is independent of binary evaluation order")
print("STAGE 262 RESULT : True")
print()

print("STAGE 263 — SCALAR MULTIPLICATION LAYER LAW")
for n in range(0,5):
    for v in product(range(4), repeat=n):
        S=encode(v)
        I=frozenset(range(n))
        for k in range(1,5):
            U=encode(scalar(v,k))
            for t in range(1,len(U)+1):
                assert layer(U,t,I)==layer(S,ceil(t/k),I)
print("  S_t(kv)=S_{ceil(t/k)}(v)")
print("  scalar multiplication repeats each Boolean layer exactly k vertical times")
print("STAGE 263 RESULT : True")
print()

print("STAGE 264 — SELF-CONVOLUTION POWER")
for n in range(0,4):
    for v in product(range(4), repeat=n):
        S=encode(v)
        for k in range(1,5):
            U=tuple()
            for _ in range(k):
                U=conv(U,S,n)
            assert U==encode(scalar(v,k))
print("  S(v)^{⋆k}=S(kv)")
print("STAGE 264 RESULT : True")
print()

print("STAGE 265 — SUPPORT IS SCALAR-INVARIANT")
for n in range(0,5):
    for v in product(range(4), repeat=n):
        S=encode(v)
        for k in range(1,5):
            U=encode(scalar(v,k))
            s1=S[0] if S else frozenset()
            u1=U[0] if U else frozenset()
            assert u1==s1
print("  for k>=1, supp(kv)=supp(v)")
print("STAGE 265 RESULT : True")
print()

print("STAGE 266 — HEIGHT SCALES UNDER SELF-ADDITION")
for n in range(0,5):
    for v in product(range(4), repeat=n):
        h=max(v,default=0)
        for k in range(1,6):
            assert max(scalar(v,k),default=0)==k*h
print("  h(kv)=k h(v)")
print("STAGE 266 RESULT : True")
print()

print("STAGE 267 — LAYER AREA IS ADDITIVE")
for n in range(0,5):
    for v in product(range(4), repeat=n):
        for w in product(range(4), repeat=n):
            assert stack_mass(encode(add(v,w)))==stack_mass(encode(v))+stack_mass(encode(w))
print("  sum_t |S_t(v+w)| = sum_t |S_t(v)| + sum_t |S_t(w)|")
print("  Boolean carry redistributes vertical mass but does not create or destroy it")
print("STAGE 267 RESULT : True")
print()

print("STAGE 268 — BINARY-INDICATOR ADDITION")
for n in range(0,6):
    I=set(range(n))
    for maskA in range(1<<n):
        A=frozenset(i for i in I if (maskA>>i)&1)
        for maskB in range(1<<n):
            B=frozenset(i for i in I if (maskB>>i)&1)
            U=encode(add(indicator(A,n),indicator(B,n)))
            assert layer(U,1,frozenset(I))==A|B
            assert layer(U,2,frozenset(I))==A&B
            assert len(U)<=2
print("  1_A + 1_B <-> layers (A union B, A intersect B)")
print("  Boolean overlap is literally the carry bit")
print("STAGE 268 RESULT : True")
print()

print("STAGE 269 — DISJOINT BINARY INPUTS HAVE NO CARRY")
for n in range(0,6):
    I=set(range(n))
    for maskA in range(1<<n):
        A=frozenset(i for i in I if (maskA>>i)&1)
        B=frozenset(I-set(A))
        U=encode(add(indicator(A,n),indicator(B,n)))
        assert len(U)<=1
        if n:
            assert U[0]==frozenset(I)
print("  if A intersect B=empty, 1_A+1_B remains squarefree")
print("STAGE 269 RESULT : True")
print()

print("STAGE 270 — K-ARY BOOLEAN ADDITION CHECKPOINT")
print("  addition of many coordinate vectors is an exact Boolean convolution")
print("  scalar multiplication vertically repeats layers")
print("  binary squarefree addition produces union at level 1 and intersection carry at level 2")
print("STAGE 270 RESULT : True")
print()

# ---------------------------------------------------------------------------
print("STAGE 271 — CAPPED ADDITION IS LAYER TRUNCATION")
for n in range(0,5):
    for v in product(range(4), repeat=n):
        for w in product(range(4), repeat=n):
            U=encode(add(v,w))
            for d in range(0,5):
                capped=encode(clamp(add(v,w),d))
                assert capped==U[:d]
print("  encode(min(d,v+w)) = first d layers of encode(v+w)")
print("STAGE 271 RESULT : True")
print()

print("STAGE 272 — OVERFLOW IS THE CONVOLUTION TAIL")
for n in range(0,5):
    for v in product(range(4), repeat=n):
        for w in product(range(4), repeat=n):
            s=add(v,w)
            U=encode(s)
            for d in range(0,5):
                defect=tuple(max(x-d,0) for x in s)
                assert encode(defect)==U[d:]
print("  overflow defect max(v+w-d,0) has layers U_{d+1},U_{d+2},...")
print("STAGE 272 RESULT : True")
print()

print("STAGE 273 — EXACT CAP/OVERFLOW SPLITTING")
for n in range(0,5):
    for s in product(range(8), repeat=n):
        for d in range(0,5):
            c=clamp(s,d)
            o=tuple(max(x-d,0) for x in s)
            assert add(c,o)==s
print("  s=min(d,s)+max(s-d,0) coordinatewise")
print("  exact value splits into retained lower layers AND overflow tail")
print("STAGE 273 RESULT : True")
print()

print("STAGE 274 — SATURATION CRITERION")
for n in range(0,5):
    for s in product(range(7), repeat=n):
        U=encode(s)
        for d in range(0,5):
            saturated=(clamp(s,d)!=s)
            assert saturated==(len(U)>d)
print("  saturation occurs iff the exact convolution has a nonempty layer above d")
print("STAGE 274 RESULT : True")
print()

print("STAGE 275 — SATURATION DEFECT MASS")
for n in range(0,5):
    for s in product(range(7), repeat=n):
        U=encode(s)
        for d in range(0,5):
            defect=tuple(max(x-d,0) for x in s)
            assert sum(defect)==sum(len(A) for A in U[d:])
print("  ||overflow||_1 = sum_{t>d} |U_t|")
print("STAGE 275 RESULT : True")
print()

print("STAGE 276 — CAPPED IDEMPOTENTS AS CONSTANT STACKS")
for n in range(0,5):
    for d in range(1,5):
        for v in product(range(d+1), repeat=n):
            idem=(clamp(add(v,v),d)==v)
            S=encode(v)
            constant=(all(x in (0,d) for x in v))
            if constant and d>0:
                support=S[0] if S else frozenset()
                assert all(A==support for A in S)
                assert len(S) in (0,d)
            assert idem==constant
print("  capped idempotent <-> every coordinate is 0 or d")
print("  layer form: S_1=...=S_d=A for one Boolean support A")
print("STAGE 276 RESULT : True")
print()

print("STAGE 277 — BOOLEAN CORNERS ARE CONSTANT-LAYER STACKS")
for n in range(0,6):
    for d in range(1,5):
        for mask in range(1<<n):
            A=frozenset(i for i in range(n) if (mask>>i)&1)
            v=tuple(d if i in A else 0 for i in range(n))
            S=encode(v)
            assert S==tuple(A for _ in range(d)) if A else S==tuple()
print("  A subseteq I maps to d·1_A <-> d repeated copies of A")
print("STAGE 277 RESULT : True")
print()

print("STAGE 278 — CAPPED ADDITION OF CORNERS IS UNION")
for n in range(0,6):
    for d in range(1,4):
        for ma in range(1<<n):
            A=frozenset(i for i in range(n) if (ma>>i)&1)
            a=tuple(d if i in A else 0 for i in range(n))
            for mb in range(1<<n):
                B=frozenset(i for i in range(n) if (mb>>i)&1)
                b=tuple(d if i in B else 0 for i in range(n))
                c=clamp(add(a,b),d)
                want=tuple(d if i in (A|B) else 0 for i in range(n))
                assert c==want
print("  d·1_A ⊕_d d·1_B = d·1_{A union B}")
print("STAGE 278 RESULT : True")
print()

print("STAGE 279 — MEET OF CORNERS IS INTERSECTION")
for n in range(0,6):
    for d in range(1,4):
        for ma in range(1<<n):
            A=frozenset(i for i in range(n) if (ma>>i)&1)
            a=tuple(d if i in A else 0 for i in range(n))
            for mb in range(1<<n):
                B=frozenset(i for i in range(n) if (mb>>i)&1)
                b=tuple(d if i in B else 0 for i in range(n))
                m=meet(a,b)
                want=tuple(d if i in (A&B) else 0 for i in range(n))
                assert m==want
print("  (d·1_A) meet (d·1_B) = d·1_{A intersect B}")
print("STAGE 279 RESULT : True")
print()

print("STAGE 280 — SATURATION/CARRY CHECKPOINT")
print("  exact convolution produces all carry layers")
print("  capped addition keeps the first d layers and discards the tail")
print("  overflow is exactly that discarded tail")
print("  capped idempotents are constant-layer Boolean corners")
print("STAGE 280 RESULT : True")
print()

# ---------------------------------------------------------------------------
print("STAGE 281 — CONVOLUTION DISTRIBUTES OVER LAYERWISE JOIN")
for n in range(0,4):
    vals=list(product(range(3), repeat=n))
    for a in vals:
        for b in vals:
            for c in vals:
                A,B,C=encode(a),encode(b),encode(c)
                assert conv(A,stack_join(B,C),n)==stack_join(conv(A,B,n),conv(A,C,n))
print("  A⋆(B∨C)=(A⋆B)∨(A⋆C)")
print("STAGE 281 RESULT : True")
print()

print("STAGE 282 — CONVOLUTION DISTRIBUTES OVER LAYERWISE MEET")
for n in range(0,4):
    vals=list(product(range(3), repeat=n))
    for a in vals:
        for b in vals:
            for c in vals:
                A,B,C=encode(a),encode(b),encode(c)
                assert conv(A,stack_meet(B,C),n)==stack_meet(conv(A,B,n),conv(A,C,n))
print("  A⋆(B∧C)=(A⋆B)∧(A⋆C)")
print("STAGE 282 RESULT : True")
print()

print("STAGE 283 — MONOTONICITY OF CONVOLUTION")
for n in range(0,4):
    vals=list(product(range(3), repeat=n))
    for a in vals:
        for b in vals:
            if not leq(a,b):
                continue
            for c in vals:
                assert stack_leq(conv(encode(a),encode(c),n),conv(encode(b),encode(c),n))
print("  A<=B implies A⋆C<=B⋆C")
print("STAGE 283 RESULT : True")
print()

print("STAGE 284 — CANCELLATION OF BOOLEAN CONVOLUTION")
for n in range(0,4):
    vals=list(product(range(3), repeat=n))
    seen={}
    for a in vals:
        for c in vals:
            key=(encode(c),conv(encode(a),encode(c),n))
            # For fixed C, A -> A⋆C is injective.
            if key in seen:
                assert seen[key]==a
            seen[key]=a
print("  A⋆C=B⋆C implies A=B")
print("  convolution monoid is cancellative")
print("STAGE 284 RESULT : True")
print()

print("STAGE 285 — DIVISIBILITY IS LAYERWISE ORDER")
for n in range(0,4):
    vals=list(product(range(4), repeat=n))
    for a in vals:
        for b in vals:
            divides=leq(a,b)
            layerorder=stack_leq(encode(a),encode(b))
            assert divides==layerorder
            if divides:
                q=sub(b,a)
                assert conv(encode(a),encode(q),n)==encode(b)
print("  A divides_⋆ B iff A_t subseteq B_t for every layer t")
print("  unique quotient is encode(b-a)")
print("STAGE 285 RESULT : True")
print()

print("STAGE 286 — GCD/LCM ARE LAYERWISE INTERSECTION/UNION")
for n in range(0,5):
    for a in product(range(4), repeat=n):
        for b in product(range(4), repeat=n):
            assert encode(meet(a,b))==stack_meet(encode(a),encode(b))
            assert encode(join(a,b))==stack_join(encode(a),encode(b))
print("  gcd_⋆ = layerwise intersection")
print("  lcm_⋆ = layerwise union")
print("STAGE 286 RESULT : True")
print()

print("STAGE 287 — SQUAREFREE ELEMENTS ARE ONE-LAYER STACKS")
for n in range(0,5):
    for v in product(range(3), repeat=n):
        squarefree=all(x<=1 for x in v)
        assert squarefree==(len(encode(v))<=1)
print("  valuation vector is {0,1}-valued iff its stack has height <=1")
print("STAGE 287 RESULT : True")
print()

print("STAGE 288 — FACTORIZATION INTO SQUAREFREE LAYERS")
for n in range(0,5):
    for v in product(range(4), repeat=n):
        S=encode(v)
        U=tuple()
        for A in S:
            U=conv(U,encode(indicator(A,n)),n)
        assert U==S
print("  encode(v)=⋆_t encode(1_{S_t(v)})")
print("  every coordinate vector factors into its threshold squarefree layers")
print("STAGE 288 RESULT : True")
print()

print("STAGE 289 — UNIQUENESS OF DESCENDING SQUAREFREE FACTORIZATION")
for n in range(0,5):
    for v in product(range(4), repeat=n):
        S=encode(v)
        # Reconstruction by counting layer membership is unique.
        assert decode(S,n)==v
        assert encode(decode(S,n))==S
print("  requiring S_1 superseteq S_2 superseteq ... makes the squarefree layer factorization unique")
print("STAGE 289 RESULT : True")
print()

print("STAGE 290 — LAYER-MONOID CHECKPOINT")
print("  descending Boolean stacks with ⋆ are isomorphic to N^(I)")
print("  divisibility is layerwise inclusion")
print("  gcd/lcm are layerwise intersection/union")
print("  squarefree elements are one-layer stacks")
print("  every element has a unique descending squarefree-layer factorization")
print("STAGE 290 RESULT : True")
print()

# ---------------------------------------------------------------------------
print("STAGE 291 — ATOMIC COORDINATE FACTORIZATION")
for n in range(0,5):
    for v in product(range(4), repeat=n):
        rec=[0]*n
        for i,m in enumerate(v):
            rec[i]+=m
        assert tuple(rec)==v
print("  v=sum_i v_i e_i is the unique free-commutative atom factorization")
print("STAGE 291 RESULT : True")
print()

print("STAGE 292 — TWO CANONICAL NORMAL FORMS")
print("  horizontal/atomic: v=sum_i v_i e_i")
print("  vertical/layered:  v=sum_t 1_{S_t(v)}")
print("  both are lossless; one groups by coordinate, the other by multiplicity threshold")
print("STAGE 292 RESULT : True")
print()

print("STAGE 293 — DISCRETE CAVALIERI / FUBINI IDENTITY")
for n in range(0,6):
    for v in product(range(5), repeat=n):
        assert sum(v)==sum(len(A) for A in encode(v))
print("  sum_i v_i = sum_t |S_t(v)|")
print("  total coordinate mass equals Boolean layer area")
print("STAGE 293 RESULT : True")
print()

print("STAGE 294 — WEIGHTED LAYER-CAKE IDENTITY")
for n in range(0,5):
    weights=tuple(i+1 for i in range(n))
    for v in product(range(5), repeat=n):
        lhs=sum(c*x for c,x in zip(weights,v))
        rhs=sum(sum(weights[i] for i in A) for A in encode(v))
        assert lhs==rhs
print("  sum_i c_i v_i = sum_t sum_{i in S_t(v)} c_i")
print("STAGE 294 RESULT : True")
print()

print("STAGE 295 — INNER PRODUCT AS ALL-LAYER OVERLAP")
for n in range(0,5):
    vals=list(product(range(4), repeat=n))
    for v in vals:
        S=encode(v)
        for w in vals:
            T=encode(w)
            rhs=sum(len(A&B) for A in S for B in T)
            assert dot(v,w)==rhs
print("  <v,w> = sum_{a>=1,b>=1} |S_a(v) intersect S_b(w)|")
print("STAGE 295 RESULT : True")
print()

print("STAGE 296 — MIN/MAX AS ALIGNED LAYER OVERLAP/UNION")
for n in range(0,5):
    vals=list(product(range(4), repeat=n))
    for v in vals:
        S=encode(v)
        for w in vals:
            T=encode(w)
            H=max(len(S),len(T))
            minmass=0; maxmass=0
            for t in range(H):
                A=S[t] if t<len(S) else frozenset()
                B=T[t] if t<len(T) else frozenset()
                minmass+=len(A&B)
                maxmass+=len(A|B)
            assert sum(meet(v,w))==minmass
            assert sum(join(v,w))==maxmass
print("  ||v∧w||_1=sum_t |S_t intersect T_t|")
print("  ||v∨w||_1=sum_t |S_t union T_t|")
print("STAGE 296 RESULT : True")
print()

print("STAGE 297 — L1 DISTANCE AS SYMMETRIC-DIFFERENCE AREA")
for n in range(0,5):
    vals=list(product(range(4), repeat=n))
    for v in vals:
        S=encode(v)
        for w in vals:
            T=encode(w)
            H=max(len(S),len(T))
            rhs=0
            for t in range(H):
                A=S[t] if t<len(S) else frozenset()
                B=T[t] if t<len(T) else frozenset()
                rhs+=len(A^B)
            assert l1(sub(v,w))==rhs
print("  d_1(v,w)=sum_t |S_t(v) symmetric_difference S_t(w)|")
print("  coordinate Hasse distance is exactly Boolean-layer symmetric-difference area")
print("STAGE 297 RESULT : True")
print()

print("STAGE 298 — RANK VALUATION / INCLUSION-EXCLUSION")
for n in range(0,5):
    vals=list(product(range(4), repeat=n))
    for v in vals:
        for w in vals:
            assert sum(v)+sum(w)==sum(meet(v,w))+sum(join(v,w))
print("  rho(v)+rho(w)=rho(v∧w)+rho(v∨w)")
print("  layerwise this is ordinary |A|+|B|=|A∩B|+|A∪B|")
print("STAGE 298 RESULT : True")
print()

print("STAGE 299 — MEDIAN AS LAYERWISE BOOLEAN MAJORITY")
for n in range(0,4):
    vals=list(product(range(4), repeat=n))
    for a in vals:
        A=encode(a)
        for b in vals:
            B=encode(b)
            for c in vals:
                C=encode(c)
                m=tuple(sorted((x,y,z))[1] for x,y,z in zip(a,b,c))
                M=encode(m)
                H=max(len(A),len(B),len(C))
                for t in range(1,H+1):
                    At=A[t-1] if t<=len(A) else frozenset()
                    Bt=B[t-1] if t<=len(B) else frozenset()
                    Ct=C[t-1] if t<=len(C) else frozenset()
                    Mt=M[t-1] if t<=len(M) else frozenset()
                    assert Mt==majority3(At,Bt,Ct)
print("  S_t(median(a,b,c)) = majority(S_t(a),S_t(b),S_t(c))")
print("  Stage-229 median geometry becomes layerwise Boolean majority")
print("STAGE 299 RESULT : True")
print()

print("STAGE 300 — BOOLEAN CARRY ALGEBRA CHECKPOINT")
print("  universal layer object:")
print("    finite descending Boolean stacks over P(I)")
print()
print("  lattice structure:")
print("    join = layerwise union")
print("    meet = layerwise intersection")
print()
print("  monoid structure:")
print("    addition transported to Boolean convolution ⋆")
print("    (S⋆T)_t=union_{a+b=t}(S_a intersect T_b)")
print()
print("  carry interpretation:")
print("    layer 1 = support union")
print("    layer 2 adds support overlap")
print("    higher layers collect all depth-overlap carries")
print()
print("  saturation:")
print("    cap d = truncate after d layers")
print("    overflow = discarded convolution tail")
print()
print("  normal forms:")
print("    atomic by coordinates")
print("    descending squarefree by Boolean thresholds")
print()
print("  geometry/statistics:")
print("    mass = layer area")
print("    inner product = all-layer overlap")
print("    L1 distance = aligned symmetric-difference area")
print("    median = layerwise Boolean majority")
print()
print("  applications:")
print("    provenance: layers are suffix envelopes")
print("    append: layers are multiplicity-threshold squarefree factors")
print()
print("PARACONSISTENT LANDING")
print("  the common Boolean skeleton now carries not only lattice order")
print("  AND the full additive monoid through an exact carry convolution.")
print("  union/intersection describe horizontal Boolean logic")
print("  AND vertical convolution restores multiplicity/depth arithmetic.")
print()
print("STAGE 300 RESULT : True")
print()
print("BATCH 251–300 RESULT : True")
