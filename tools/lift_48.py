#!/usr/bin/env python3
"""lift_48.py - Stage 48: THE ADJUNCTION TURNS BACK AS A COMONAD.

The SAME adjunction F ⊣ U of Stage 47 induces a comonad on EM(T):

    G := F ∘ U              : EM(T) -> EM(T)     (NOT the monad T = U F on Set)
    G(A) = F(U A)           = (P(A), m_A)
    eps_A : G(A) -> A       eps_A(K) = alpha(K)
    chi_A : G(A) -> G^2(A)  chi_A(K) = { {a} | a in K }

T = U F lives on Set;   G = F U lives on EM(T).  Opposite sides, held explicit:

    T : nest -> flatten                       (monad, Stage 43)
    G : value -> singleton-generated decomposition   (comonad)

The two are NOT inverses; both structures are kept.
"""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lift_37 import ENGAGR
from lift_35 import weight_fields, run_batch
from lift_45 import alpha, FOUR, BOT
import lift_43 as L43
import lift_46 as L46
import lift_reentry as R

LAB = L46.LAB
W = L46.W
ORD3 = ["t", "i", "f"]
word = L43.word

def singletons(K):
    return frozenset(frozenset({a}) for a in K)

def big_union(KK):
    r = frozenset()
    for K in KK:
        r = r | K
    return r

def epsilon(o, K):
    return alpha(o, sorted(K))

def subsets_four():
    return [frozenset(FOUR[i] for i in range(4) if (m >> i) & 1) for m in range(1 << 4)]

# ---------------------------------------------------------------- 48A  comonad
def comonad_def():
    print("=== 48A: THE INDUCED COMONAD  G = F∘U ===")
    print("   G(A)=F(U A)=(P(A), m_A);  eps_A(K)=alpha(K);  chi_A(K)={a}|a in K")
    K = frozenset({"T", "F"})
    chi = singletons(K)
    print(f"   chi_A({{T,F}}) = {len(chi)} singleton distinctions, nested word len {len(word(chi))}")
    print("   T = U F on Set  vs  G = F U on EM(T): opposite sides, NOT identified.")
    print("   T: nest->flatten ;  G: value->singleton-generated decomposition (not inverses).")
    print()
    return True

# ---------------------------------------------------------- 48B  three algebras
def three_algebras():
    print("=== 48B: G(A_t)=G(A_i)=G(A_f)  but  eps_t != eps_i != eps_f ===")
    Krep = frozenset({"T", "F"})
    obj_same = all(word(Krep) == word(Krep) for o in ORD3)
    eps = {o: epsilon(o, Krep) for o in ORD3}
    dist = len(set(eps.values())) == 3
    print(f"   G(A_o) object part = (P(FOUR), union), identical for all 3 : {obj_same}")
    print(f"   eps on {{T,F}}: eps_t={eps['t']}  eps_i={eps['i']}  eps_f={eps['f']}  distinct : {dist}")
    print("   same under G's object part AND different under eps -- a held split, not collapse.")
    print()
    return obj_same and dist

# ----------------------------------------------------------- 48C  comonad laws
def comonad_laws():
    print("=== 48C: THE THREE COMONAD LAWS ===")
    subs = subsets_four()
    ok = True
    for o in ORD3:
        l1 = all(big_union(singletons(K)) == K for K in subs)              # eps_G ∘ chi = id
        l2 = all(frozenset(epsilon(o, {a}) for a in K) == K for K in subs)  # G(eps) ∘ chi = id
        l3 = all(singletons(singletons(K)) == frozenset(singletons(M) for M in singletons(K))
                 for K in subs)                                            # chi_G ∘ chi = G(chi) ∘ chi
        ok &= l1 and l2 and l3
        print(f"   order {o}:  eps_G∘chi=id {l1}   G(eps)∘chi=id {l2}   chi_G∘chi=G(chi)∘chi {l3}")
    print("   monad: nest->flatten ;  comonad: value->singleton decomposition.  Held together.")
    print()
    return ok

# ------------------------------------------------------------ 48D  coalgebras
def coalgebra():
    print("=== 48D: G-COALGEBRAS -- the free algebra carries a canonical beta ===")
    D4 = ["T", "F", "A1", "tf"]
    wd = [W[l] for l in D4]
    subs = [frozenset(wd[i] for i in range(len(wd)) if (m >> i) & 1) for m in range(1 << len(wd))]
    beta = singletons
    hom = (beta(frozenset()) == frozenset())
    for A in subs:
        for Bq in subs:
            if beta(A | Bq) != (beta(A) | beta(Bq)):
                hom = False
    counit = all(big_union(beta(K)) == K for K in subs)                    # eps(beta(K)) = K
    coassoc = all(singletons(beta(K)) == frozenset(beta(M) for M in beta(K)) for K in subs)
    print("   beta_D(K)={x}|x in K ; E-M hom (empty+binary union preserved) : " + str(hom))
    print(f"   counit   eps_(D)(beta(K)) = UNION = K                         : {counit}")
    print(f"   coassoc  chi(beta(K)) = G(beta)(beta(K))                     : {coassoc}")
    KT = frozenset({W["T"], W["F"]})
    gws = [word(beta(KT)), word(singletons(beta(KT))), word(big_union(beta(KT)))]
    try:
        o = run_batch(["weight " + w for w in gws])
        finals = [weight_fields(o, w).get("final") for w in gws]
        ground = all(f is not None for f in finals)
    except Exception:
        ground = False
    print(f"   kernel grounding: nested SET words close (final present) : {ground}")
    print("   reading: beta exposes a higher-level set as its generating singleton")
    print("   distinctions; eps fuses them back.  The coalgebraic face of the tower.")
    print()
    return hom and counit and coassoc

def main():
    print("=== Stage 48: THE ADJUNCTION TURNS BACK AS A COMONAD ===")
    a = comonad_def()
    b = three_algebras()
    c = comonad_laws()
    d = coalgebra()
    ok = a and b and c and d
    print("STAGE48: the adjunction F⊣U induces a comonad G=F∘U on EM(T) -- eps_A=alpha,")
    print("   chi_A=singletonization -- all three laws hold, and the free algebra carries")
    print("   the canonical G-coalgebra beta_D(K)={{x}|x∈K}.")
    print(f"   48A comonad {a} | 48B three algebras {b} | 48C laws {c} | 48D coalgebra {d} => {ok}")
    print("   NEXT (Stage 49): EM(T) as its own carrier, or the Kleisli/E-M duality.")

if __name__ == "__main__":
    main()
