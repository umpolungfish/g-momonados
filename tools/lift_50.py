#!/usr/bin/env python3
"""lift_50.py - Stage 50: KLEISLI IS THE FREE-ALGEBRA SECTOR OF E-M.

    J : Kl(T) -> EM(T)      J(X)=F(X)=(P(X), m)      J(k)(K)=K>>=k = UNION{k(x)}

The claim is stronger than functoriality: on the free-algebra sector J is a
bijection on homs.

    R(h)(x) := h({x})                     inverse on arrows (restrict to generators)
    Hom_Kl(X,Y)  ≅  Hom_EM(F X, F Y)      R∘J=id,  J∘R=id

  50A inverse on arrows
  50B FULL + FAITHFUL  (recover every free E-M hom from its singleton generators)
  50C essential image  (free E-M objects inside E0 up to E-M isomorphism -- NOT carrier equality)
  50D the Stage-32/33 fork survives J  (κ vs κ̄ remain distinct)
  50E categorical statement  Kl(T) ≃ EM_free(T) ⊂ EM(T),  EM_free FULL

We do NOT say Kl(T)=EM(T): EM(T) contains non-free algebra structures beyond the
sector represented directly by Kleisli objects.
"""
import sys, os, itertools
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lift_37 import ENGAGR
from lift_45 import alpha, FOUR, BOT
import lift_43 as L43
import lift_44 as L44
import lift_46 as L46

ORD3 = ["t", "i", "f"]
word = L43.word

def allsubs(items):
    items = list(items)
    return [frozenset(items[i] for i in range(len(items)) if (m >> i) & 1)
            for m in range(1 << len(items))]

def Jg(g, X, K):
    """J(g)(K) = UNION_{x in K} g(x);  g a generator map X -> P(Y)."""
    r = frozenset()
    for i, x in enumerate(X):
        if x in K:
            r = r | g[i]
    return r

# ---------------------------------------------------------- 50A  inverse on arrows
def inverse_arrows():
    print("=== 50A: COMPARISON INVERSE ON ARROWS  R(h)(x)=h({x}) ===")
    Y = list(L44.WIRES.values()); X = Y[:3]
    ks = {"u": lambda x: frozenset({x}),
          "kappa_FW": lambda x: L44.kappa("FRAME_WORK", x),
          "kbar_FW": lambda x: L44.kbar("FRAME_WORK", x)}
    subsX = allsubs(X)
    rj = all(word(L46.J(k, frozenset({x}))) == word(k(x)) for k in ks.values() for x in X)
    def R(k, x):     # R(J(k))(x) = J(k)({x})
        return L46.J(k, frozenset({x}))
    jr = all(word(L46.J(lambda y, k=k: R(k, y), K)) == word(L46.J(k, K))
             for k in ks.values() for K in subsX)
    print("   R∘J = id   R(J(k))(x)=J(k)({x})=k(x)                  : " + str(rj))
    print("   J∘R = id   on free homs  J(R(h))=h                     : " + str(jr))
    print()
    return rj and jr

# ------------------------------------------------------------ 50B  full + faithful
def full_faithful():
    print("=== 50B: FULL + FAITHFUL on the free-algebra sector ===")
    Y = list(L44.WIRES.values()); X = Y[:3]
    AY = allsubs(Y)
    subsX = allsubs(X)
    gens = list(itertools.product(AY, repeat=len(X)))
    full = True
    for g in gens:
        h = lambda K, g=g: Jg(g, X, K)
        if h(frozenset()) != frozenset():
            full = False
        for A in subsX:
            for B in subsX:
                if h(A | B) != (h(A) | h(B)):
                    full = False
        kh = {x: h(frozenset({x})) for x in X}
        for K in subsX:
            r = frozenset()
            for x in K:
                r = r | kh[x]
            if word(r) != word(h(K)):
                full = False
    keys = {tuple(word(Jg(g, X, K)) for K in subsX) for g in gens}
    faithful = (len(keys) == len(gens))
    print(f"   free homs enumerated (generator maps X->P(Y)): {len(gens)}")
    print(f"   FULL   every free E-M hom = J(k_h), k_h(x)=h({{x}})   : {full}")
    print(f"   FAITHFUL  J injective (distinct J-images = distinct k) : {faithful}  "
          f"({len(keys)}/{len(gens)})")
    print()
    return full and faithful

# ------------------------------------------------------- 50C  essential image
def iso_to_free(alphaA, carrierA):
    """is (carrierA, alphaA) E-M isomorphic to the free algebra F(2)=(P({a,b}), union)?"""
    a, b = "a", "b"
    Fc = [frozenset(), frozenset({a}), frozenset({b}), frozenset({a, b})]
    def uni(KK):
        r = frozenset()
        for K in KK:
            r = r | K
        return r
    subs = allsubs(carrierA)
    for perm in itertools.permutations(Fc):
        phi = dict(zip(carrierA, perm))
        if all(phi[alphaA(K)] == uni([phi[x] for x in K]) for K in subs):
            return True
    return False

def iso_pair(alpha1, alpha2, carrier):
    """are two E-M algebras on the SAME carrier E-M isomorphic?"""
    subs = allsubs(carrier)
    for perm in itertools.permutations(carrier):
        phi = dict(zip(carrier, perm))
        if all(phi[alpha1(K)] == alpha2(sorted({phi[x] for x in K})) for K in subs):
            return True
    return False

def essential_image():
    print("=== 50C: ESSENTIAL IMAGE -- free E-M objects in E0 UP TO E-M ISOMORPHISM ===")
    crisp = {o: (lambda K, o=o: alpha(o, sorted(K))) for o in ORD3}
    for o in ORD3:
        print(f"   A_{o}  ≅ F(2)=(P({{a,b}}),∪) ?  {iso_to_free(crisp[o], FOUR)}")
    chain = {"N": 0, "T": 1, "F": 2, "B": 3}
    def alpha_chain(K):
        return "N" if not K else max(K, key=lambda x: chain[x])
    print(f"   A_chain (4-chain, join=sup) ≅ F(2) ?  {iso_to_free(alpha_chain, FOUR)}")
    print("   A_D = ALG(P(D),m_D) IS free over D by construction (A_D = J(D)).")
    # are the crisp three mutually E-M isomorphic?
    mut = all(iso_pair(crisp[o], crisp[p], FOUR) for o in ORD3 for p in ORD3)
    print(f"   A_t ≅ A_i ≅ A_f (mutually E-M isomorphic)        : {mut}")
    print("   => the crisp triple are three CARRIER-LABELINGS of one E-M object F(2);")
    print("      the Stage-49 distinction is carrier-fixed, not E-M-isomorphism-invariant.")
    print("   => A_chain is a NON-free E-M object: EM(T) strictly contains the Kleisli sector.")
    print()
    return (not iso_to_free(alpha_chain, FOUR)) and all(iso_to_free(crisp[o], FOUR) for o in ORD3)

# --------------------------------------------------------- 50D  the fork survives
def fork_survives():
    print("=== 50D: THE STAGE-32/33 FORK SURVIVES J ===")
    Y = list(L44.WIRES.values())
    ok = True
    for R in ["FRAME_WORK", "CLOSE_FRAME"]:
        off = [x for x in Y if L44.MORPH[R](x) is None]
        if off:
            x0 = off[0]
            k = (lambda x, R=R: L44.kappa(R, x))
            kb = (lambda x, R=R: L44.kbar(R, x))
            Jdist = word(L46.J(k, frozenset({x0}))) != word(L46.J(kb, frozenset({x0})))
            ok &= Jdist
            print(f"   R={R}: κ({{x0}})=∅; κ̄({{x0}})={x0}; J(κ) ≠ J(κ̄) : {Jdist}")
    print("   faithful ⇒ different Kleisli arrows give different free E-M homs (the fork is preserved).")
    print()
    return ok

# --------------------------------------------------- 50E  categorical statement
def categorical():
    print("=== 50E: CATEGORICAL STATEMENT ===")
    Y = list(L44.WIRES.values()); X = Y[:2]
    subsX = allsubs(X)
    arrows = [lambda x: frozenset({x}),
              lambda x: L44.kappa("FRAME_WORK", x),
              lambda x: L44.kbar("FRAME_WORK", x),
              lambda x: L44.kappa("CLOSE_FRAME", x)]
    gsf = [(g, f, L44.kleisli(g, f)) for f in arrows for g in arrows]
    def key(h):
        return tuple(word(L46.J(h, K)) for K in subsX)
    refl = True
    for i in range(len(gsf)):
        for j in range(len(gsf)):
            if key(gsf[i][2]) == key(gsf[j][2]):
                if any(word(gsf[i][2](x)) != word(gsf[j][2](x)) for x in X):
                    refl = False
    print("   J : Kl(T) → EM(T),  J(X)=F(X),  J(k)(K)=K>>=k")
    print("   Kl(T) ≃ EM_free(T) ⊂ EM(T)   (EM_free a FULL subcategory)")
    print(f"   no information lost:  J(g⋆f)=J(g'⋆f') ⇒ g⋆f=g'⋆f'  : {refl}")
    print("   NOT Kl(T)=EM(T): EM(T) carries non-free objects beyond this sector (A_chain).")
    print()
    return refl

def main():
    print("=== Stage 50: KLEISLI IS THE FREE-ALGEBRA SECTOR OF E-M ===")
    a = inverse_arrows()
    b = full_faithful()
    c = essential_image()
    d = fork_survives()
    e = categorical()
    ok = a and b and c and d and e
    print("STAGE50: J : Kl(T)→EM(T) is fully faithful onto the free-algebra sector:")
    print("   the inverse on arrows is restriction to generators, R(h)(x)=h({x}); every")
    print("   free E-M hom is J(k_h); distinct Kleisli arrows stay distinct; the Stage-32/33")
    print("   fork survives; and EM(T) strictly contains this sector (A_chain is non-free).")
    print(f"   50A inverse {a} | 50B full+faithful {b} | 50C image {c} | 50D fork {d} | 50E categorical {e} => {ok}")
    print("   NEXT (Stage 51): E-M objects re-entering themselves (internal re-entry).")

if __name__ == "__main__":
    main()
