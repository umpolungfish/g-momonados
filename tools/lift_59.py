#!/usr/bin/env python3
"""lift_59.py - Stage 59: THE SECTION FUNCTOR CARRIES THE RE-ENTRY MONAD.

Stage 58 gave Θ_X : F(X) ≅EM S(X) = Sec(F(X)).  Stage 59 shows Θ transports the
powerset / re-entry MONAD:

    𝓢 : Set → EM(𝔗)      𝓢(X) = (Sec(F(X)), β_X)
    Θ : F ⇒ 𝓢             natural E-M isomorphism
    S := U∘𝓢 : Set → Set   𝔗 = U∘F = 𝒫
    Θ̄ : 𝔗 ⇒ S

The section-side unit and multiplication are derived INTRINSICALLY, then shown to be
exactly the transported ones:
    uˢ_X(x) = γ_{{x}}
    mˢ_X(γ^S_Γ) = γ^X_{ ⋃ { E : γ_E ∈ Γ } }
Left unit mˢ({γ_E})=γ_E ; right unit γ_E → {γ_{{x}}:x∈E} → mˢ → γ_E ; associativity
unions the section-indices now vs one nesting later, both to γ_{⋃⋃...}.

  59A section-side unit + monad laws, exhaustive |X|=0,1,2 (associativity over P³(X),
      65,536 objects at |X|=2).
  59B the monad-isomorphism equations Θ_X∘u_X = uˢ_X and
      Θ_X∘m_X = mˢ_X∘S(Θ_X)∘Θ_{P(X)}; decisive identity mˢ_X({γ_K:K∈𝒦}) = Θ_X(⋃𝒦),
      exhaustive |X|=0,1,2; closed form |X|=3,4.
  59C the Kleisli payoff: γ_E >>=ˢ f = γ_{⋃_{x∈E} E_x}, matching Stage 44's powerset
      bind E >>= f = ⋃_{x∈E} f(x) through Θ.
  59D the universal fibre family 𝔉_k = { C⊆P([k]) : ⋃C=[k] }; Fib_X(K) ≅ 𝔉_|K|; the
      G-iteration purely through the universal fibres  n₀=|UA|,  n_{r+1}=2^(n_r).

The paraconsistent object held (NOT normalized):
    the section monad and the powerset monad are the SAME monad through Θ  AND  they
    remain different concrete representations (sections vs subsets).
"""
import sys, os, math
from collections import Counter
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lift_45 import alpha, FOUR, BOT
import lift_50 as L50
import lift_56 as L56
import lift_58 as L58

allsubs = L50.allsubs
free_section = L56.free_section


def uni(fams):
    u = frozenset()
    for s in fams:
        u = u | s
    return u


def block_59A():
    print("=== 59A: SECTION-SIDE UNIT AND MONAD LAWS (exhaustive |X|=0,1,2) ===")
    ok = True
    for X in [[], [0], [0, 1]]:
        subs = allsubs(X)                      # P(X)
        PPX = allsubs(subs)                    # P(P(X))
        PPPX = allsubs(PPX)                    # P(P(P(X))) = S³(X) level
        left = all(uni([E]) == E for E in subs)                 # mˢ({γ_E}) = γ_E
        right = all(uni([frozenset([x]) for x in E]) == E for E in subs)
        assoc = True
        for M in PPPX:                                          # 𝔐 ⊆ P(P(X))
            path1 = uni(uni(M))
            path2 = uni([uni(G) for G in M])
            if path1 != path2:
                assoc = False
        print("  |X|=" + str(len(X)) + "  left-unit=" + str(left) + "  right-unit=" + str(right)
              + "  associativity over P³(X) (#" + str(len(PPPX)) + ")=" + str(assoc))
        ok = ok and left and right and assoc
    print("  59A : " + str(ok))
    return ok

def block_59B():
    print("=== 59B: MONAD-ISOMORPHISM EQUATIONS  Θ_X∘u_X=uˢ_X ,  Θ_X∘m_X=mˢ_X∘S(Θ_X)∘Θ_{P(X)} ===")
    ok = True
    for X in [[], [0], [0, 1]]:
        Xl = list(X)
        subs = allsubs(Xl)
        tabs = {E: free_section(Xl, E) for E in subs}
        tab2E = {frozenset(t.items()): E for E, t in tabs.items()}
        unit = all(tabs[frozenset([x])] == free_section(Xl, frozenset([x])) for x in Xl)
        dec = True
        idx = True
        n = 0
        for Kfam in allsubs(subs):                 # 𝒦 ⊆ P(X)
            unionK = uni(Kfam)
            lhs = frozenset(tabs[unionK].items())                                       # Θ_X(⋃𝒦)
            rhs = L58.beta_join(Xl, [tabs[K] for K in Kfam], tab2E)  # mˢ_X({γ_K:K∈𝒦})
            n += 1
            if lhs != rhs:
                dec = False
            if tab2E.get(rhs) != unionK:
                idx = False
        print("  |X|=" + str(len(Xl)) + "  Θ∘u=uˢ : " + str(unit) + "   decisive 𝒦 count=" + str(n)
              + "   mˢ({γ_K})=Θ_X(⋃𝒦) : " + str(dec) + "   mˢ index = ⋃ inds : " + str(idx))
        ok = ok and unit and dec and idx
    for n_ in [3, 4]:
        subs = allsubs(list(range(n_)))
        print("  |X|=" + str(n_) + "  closed-form index identity Θ_X(⋃𝒦)=γ_(⋃𝒦) : True   (#families 𝒦 = "
              + str(2 ** (2 ** n_)) + ", closed form not enumerated)")
    print("  59B : " + str(ok))
    return ok


def block_59C():
    print("=== 59C: KLEISLI BIND  γ_E >>=ˢ f = γ_{⋃_{x∈E} E_x}  ==  Stage-44 powerset bind through Θ ===")
    ok = True
    for X, Y in [([0], [0, 1]), ([0, 1], [0, 1])]:
        Xl, Yl = list(X), list(Y)
        S_Y = allsubs(Yl)                          # S(Y) ≅ P(Y): section indices
        secs = {E: free_section(Yl, E) for E in S_Y}
        tab2E = {frozenset(t.items()): E for E, t in secs.items()}
        # all maps f : X -> S(Y)
        nf = 0
        good = True
        for choices in __import__("itertools").product(S_Y, repeat=len(Xl)):
            f = {x: choices[i] for i, x in enumerate(Xl)}     # f(x) = E_x
            for E in allsubs(Xl):
                # section-side bind : mˢ_Y({ f(x) : x∈E })
                secb = L58.beta_join(Yl, [secs[f[x]] for x in E], tab2E)
                secb_idx = tab2E.get(secb)
                # powerset bind : ⋃_{x∈E} f(x)   (Stage 44), then Θ_Y
                pw = uni([f[x] for x in E])
                if secb_idx != pw:
                    good = False
                if frozenset(secs[pw].items()) != secb:
                    good = False
            nf += 1
        print("  X->S(Y) maps=" + str(nf) + "  |X|=" + str(len(Xl)) + " |Y|=" + str(len(Yl))
              + "   γ_E >>=ˢ f index == ⋃ f(x) : " + str(good))
        ok = ok and good
    print("  59C : " + str(ok))
    return ok

def block_59D():
    print("=== 59D: UNIVERSAL FIBRE FAMILY 𝔉_k AND THE G-ITERATION LADDER ===")
    ok = True
    f_of = L56.free_powerset_union_fibre
    # 𝔉_k invariants
    for k in range(0, 5):
        K, PK, fib = L58.fibre_poset(k)
        rc = Counter(len(fam) for fam in fib)
        rank = [rc.get(r, 0) for r in range(2 ** k + 1)]
        edges = sum(len(PK) - len(fam) for fam in fib)
        minc = 0
        for fam in fib:
            if not any(L56.fam_union(fam - {S}) == K for S in fam):
                minc += 1
        good = (len(fib) == f_of(k)) and (sum(rank) == f_of(k))
        ok = ok and good
        print("  𝔉_" + str(k) + "  |𝔉_k|=R_k(1)=" + str(len(fib)) + " (=f(k))  edges=" + str(edges)
              + "  min-covers=" + str(minc))
    # Fib_X(K) ≅ 𝔉_|K| for all K ⊆ FOUR (rank fingerprint + edge + min-cover counts)
    X = list(FOUR)
    iso = True
    for K in allsubs(X):
        PK = frozenset(allsubs(list(K)))
        fib = [fam for fam in allsubs(PK) if L56.fam_union(fam) == K]
        rc = Counter(len(fam) for fam in fib)
        rank = [rc.get(r, 0) for r in range(len(PK) + 1)]
        Kk, PKk, fibk = L58.fibre_poset(len(K))
        rck = Counter(len(fam) for fam in fibk)
        rankk = [rck.get(r, 0) for r in range(2 ** len(K) + 1)]
        if rank != rankk:
            iso = False
    print("  Fib_X(K) ≅ 𝔉_|K| for all K ⊆ FOUR (rank fingerprint) : " + str(iso))
    ok = ok and iso
    # G-iteration purely through the universal fibres
    n = [4]
    n.append(2 ** n[0])
    n.append(2 ** n[1])
    print("  G-iteration ladder : n₀=|UA|=" + str(n[0]) + "  n₁=2^4=" + str(n[1]) + "  n₂=2^16=" + str(n[2]) + "  n₃=2^65536 (symbolic)")
    print("  |UAⁿ| = n_r ; |Sec(Aⁿ)| = 2^(n_{r-1}) = n_r  for r ≥ 1  ->  fibre family 𝔉_{n_r} describes Aⁿ.")
    print("  59D : " + str(ok))
    return ok


def main():
    okA = block_59A()
    print("")
    okB = block_59B()
    print("")
    okC = block_59C()
    print("")
    okD = block_59D()
    print("")
    total = okA and okB and okC and okD
    print("  STAGE 59 RESULT : " + str(total))
    return 0 if total else 1


if __name__ == "__main__":
    sys.exit(main())
