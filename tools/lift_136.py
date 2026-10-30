#!/usr/bin/env python3
"""
STAGE 136 — UNIQUE WEIGHT-MULTISET DECODER

Let

    A_G(x)=prod_i (1+x^{g_i})

for a finite multiset G of positive integer append weights.

Claim:
    A_G uniquely determines the multiset G.

Constructive decoder:
1. If A=1, stop.
2. Let g be the least positive exponent with nonzero coefficient.
   Then g is the smallest append weight.
3. The coefficient [x^g]A equals the multiplicity m of that smallest
   weight, because no sum of larger positive weights can equal g.
4. Divide exactly by (1+x^g)^m.
5. Recurse.

Therefore the future polynomial is a lossless code for the unordered
multiset of append weights.

It does NOT encode execution order:
    (g1,g2,...,gr)
and any permutation have the same A_G.
"""

from collections import Counter

def multiply(a,b):
    c=Counter()
    for i,ai in a.items():
        for j,bj in b.items():
            c[i+j]+=ai*bj
    return Counter({k:v for k,v in c.items() if v})

def degree(p):
    return max(p) if p else -1

def exact_div(num, den):
    assert den
    assert den[degree(den)] == 1
    rem=Counter(num)
    q=Counter()
    dd=degree(den)
    while rem and degree(rem)>=dd:
        dr=degree(rem)
        coeff=rem[dr]
        sh=dr-dd
        q[sh]+=coeff
        for k,v in den.items():
            rem[k+sh]-=coeff*v
            if rem[k+sh]==0:
                del rem[k+sh]
    assert not rem
    return Counter({k:v for k,v in q.items() if v})

def binomial_factor(g):
    return Counter({0:1,g:1})

def power_poly(base,m):
    out=Counter({0:1})
    for _ in range(m):
        out=multiply(out,base)
    return out

def poly_from_weights(weights):
    out=Counter({0:1})
    for g in weights:
        out=multiply(out,binomial_factor(g))
    return out

def decode_weights(A):
    A=Counter(A)
    assert A[0]==1
    result=[]
    while not (len(A)==1 and A.get(0,0)==1):
        positive=[k for k,v in A.items() if k>0 and v]
        assert positive
        g=min(positive)
        m=A[g]
        assert m>=1
        result.extend([g]*m)
        A=exact_div(A, power_poly(binomial_factor(g),m))
        assert A[0]==1
    return tuple(result)

families=[
    (),
    (1,),
    (1,2,4,8),
    (1,1,2),
    (2,2,2),
    (1,3,3,5),
    (2,5,9),
    (1,1,1,1,1),
    (3,6,6,10,15),
]

for ws in families:
    A=poly_from_weights(ws)
    dec=decode_weights(A)
    assert dec==tuple(sorted(ws))
    assert poly_from_weights(dec)==A

# Order forgetting.
A1=poly_from_weights((1,3,2,3))
A2=poly_from_weights((3,2,3,1))
assert A1==A2
assert decode_weights(A1)==(1,2,3,3)

print("STAGE 136 — UNIQUE WEIGHT-MULTISET DECODER")
print("="*96)
print("136A future polynomial:")
print("  A_G(x)=prod_i(1+x^{g_i})")
print()
print("136B smallest-weight decoder:")
print("  g=min positive exponent with nonzero coefficient")
print("  multiplicity(g)=[x^g]A_G")
print()
print("136C recursion:")
print("  divide by (1+x^g)^multiplicity(g)")
print("  and repeat")
print()
print("136D uniqueness theorem:")
print("  A_G uniquely determines the sorted weight multiset G")
print()
print("136E order distinction:")
print("  permutations of G produce the same A_G")
print("  so append order is not recoverable from aggregate polynomial data")
print()
print("136F executable audit:")
print("  distinct, repeated, binary, and gapped weight multisets: TRUE")
print()
print("PARACONSISTENT LANDING")
print("  aggregate future data forget execution order")
print("  AND retain the entire unordered append-weight multiset exactly.")
print()
print("STAGE 136 RESULT : True")
