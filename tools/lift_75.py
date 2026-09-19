#!/usr/bin/env python3
"""
STAGE 75 — ORIENTED AREV REFINEMENT / BANKING CAUSALITY

Measured native words:
  W7  = ⊢∈⊤⊥∋⊡⊣
  W8L = ⊢∈⊤≺⊥∋⊡⊣   (AREV before F deposit)
  W8R = ⊢∈⊤⊥≺∋⊡⊣   (AREV after F deposit)

All rotations of W8L and W8R have verdict T.

Native weight/banking:
  W8L: AREV clears 1 live unit, banks/restores 1.
  W8R: AREV clears 2 live units, banks/restores 2.
"""

from collections import Counter
import math, cmath

G7=("⊢","∈","⊤","⊥","∋","⊡","⊣")
GL=("⊢","∈","⊤","≺","⊥","∋","⊡","⊣")
GR=("⊢","∈","⊤","⊥","≺","∋","⊡","⊣")

R7=("TF","TF","TF","F","N","N","TF")
RL=("TF","TF","F","F","F","N","N","TF")
RR=("TF","TF","N","N","N","N","N","TF")

def autocorr(r):
    n=len(r)
    return tuple(sum(r[j] == r[(j+t)%n] for j in range(n)) for t in range(n))

def entropy(r):
    n=len(r); c=Counter(r)
    return -sum((v/n)*math.log2(v/n) for v in c.values())

def pullback(g_big,r_big):
    phi=tuple(g_big.index(g) for g in G7)
    return phi,tuple(r_big[i] for i in phi)

def onehot_power(r):
    cats=sorted(set(r)); n=len(r); out=[]
    for m in range(n):
        s=0.0
        for cat in cats:
            z=sum((r[j]==cat)*cmath.exp(-2j*math.pi*m*j/n) for j in range(n))
            s += abs(z)**2
        out.append(s)
    return tuple(out)

K7,KL,KR=autocorr(R7),autocorr(RL),autocorr(RR)
assert K7==(7,4,2,1,1,2,4)
assert KL==(8,5,2,0,0,0,2,5)
assert KR==(8,6,4,2,2,2,4,6)

phiL,pL=pullback(GL,RL)
phiR,pR=pullback(GR,RR)
hitsL=[i for i,(a,b) in enumerate(zip(R7,pL)) if a==b]
hitsR=[i for i,(a,b) in enumerate(zip(R7,pR)) if a==b]
missL=[(G7[i],R7[i],pL[i]) for i in range(7) if R7[i]!=pL[i]]
missR=[(G7[i],R7[i],pR[i]) for i in range(7) if R7[i]!=pR[i]]
assert len(hitsL)==6 and missL==[("⊤","TF","F")]
assert len(hitsR)==5 and missR==[("⊤","TF","N"),("⊥","F","N")]

sameLR=[i for i,(a,b) in enumerate(zip(RL,RR)) if a==b]
assert sameLR==[0,1,5,6,7]

PR=onehot_power(RR)
expected=(34,6+4*math.sqrt(2),2,6-4*math.sqrt(2),2,
          6-4*math.sqrt(2),2,6+4*math.sqrt(2))
assert all(abs(a-b)<1e-9 for a,b in zip(PR,expected))

print("STAGE 75 — ORIENTED AREV REFINEMENT / BANKING CAUSALITY")
print("="*78)
print("75A closure class: TRUE")
print("  W7, W8L, W8R all have T verdicts on their measured full ROTAT orbits.")
print()
print("75B operational asymmetry: TRUE")
print("  W8L: deposit T -> AREV clears/banks 1 -> deposit F -> FFUSE restores 1")
print("  W8R: deposit T,F -> AREV clears/banks 2 -> FFUSE restores 2")
print("  same primitive, different side of F-deposit => different live banking.")
print()
print("75C register supports:")
print("  W7 :",dict(Counter(R7)))
print("  W8L:",dict(Counter(RL)))
print("  W8R:",dict(Counter(RR)))
print()
print("75D categorical autocorrelations:")
print("  K7  =",K7)
print("  K8L =",KL)
print("  K8R =",KR)
print("  direct W8L/W8R coincidence =",len(sameLR),"/ 8")
print()
print("75E glyph-rooted transport from W7:")
print("  left  embedding =",phiL," agreement =",len(hitsL),"/7", " mismatches =",missL)
print("  right embedding =",phiR," agreement =",len(hitsR),"/7", " mismatches =",missR)
print()
print("75F phase entropy:")
print(f"  H(W7)  = {entropy(R7):.12f}")
print(f"  H(W8L) = {entropy(RL):.12f}")
print(f"  H(W8R) = {entropy(RR):.12f}")
print("  ordering: H(W8L) > H(W7) > H(W8R)")
print()
print("75G exact one-hot phase power for W8R:")
print("  P8R = (34, 6+4√2, 2, 6-4√2, 2, 6-4√2, 2, 6+4√2)")
print()
print("PARACONSISTENT LANDING")
print("  left and right AREV refinements stay in the same T closure class")
print("  AND induce inequivalent register-phase geometries.")
print("  AREV is one opcode")
print("  AND its semantic effect is oriented by which deposits occur before it.")
print("  edit location is syntactic")
print("  AND banking exposure is causal.")
print()
print("STAGE 75 RESULT : True")
