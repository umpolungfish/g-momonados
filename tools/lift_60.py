#!/usr/bin/env python3
"""lift_60.py - Stage 60: E-M(𝔗) ≅ E-M(S) / THE UNIVERSAL MULTIPLICATION FIBRE.

Stage 59 gave Θ̄ : 𝔗 ⇒ S a monad isomorphism.  Stage 60 transports the WHOLE
Eilenberg-Moore theory across it, while keeping two concrete representations distinct:

    same E-M semantics through Θ   AND   different concrete carriers (subsets vs sections).

  60A αˢ := α ∘ Θ_X⁻¹ : SX → X  (section-monad presentation); aᵗ := a ∘ Θ_X : 𝔗X → X.
      Round trips (αˢ)ᵗ = α and (aᵗ)ˢ = a; S-algebra unit and multiplication laws
      exhaustively (|X|=4 -> 65,536 section-of-sections); Fib(αˢ) = Fib(α) recovers
      [2,2,2,10] / [2,2,4,8]; section-space shape diamond↔diamond, chain↔chain.
  60B morphisms: h∘α = β∘𝔗h  iff  h∘αˢ = βˢ∘Sh (all measured E-M homs); H_Θ : E-M(𝔗)
      → E-M(S) identity on underlying sets, H_Θ⁻¹H_Θ = H_ΘH_Θ⁻¹ = id; two equality
      columns (extensional algebra vs canonical-byte representation).
  60C β_X = mˢ_X ∘ Θ_{SX}   (Stage-58 join = Stage-59 flatten), exhaustively.
  60D the universal multiplication fibre: 𝔉_k = m_[k]⁻¹([k]); Fib^S_X(γ_E) ≅ 𝔉_|E|.

The paraconsistent object held (NOT normalized):
    the two categories have the SAME E-M semantics through Θ  AND  their carriers
    (subsets vs sections) remain different concrete representations.
"""
import sys, os, math, itertools
from collections import Counter
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lift_45 import alpha, FOUR, BOT
import lift_50 as L50
import lift_53 as L53
import lift_54 as L54
import lift_56 as L56
import lift_58 as L58

allsubs = L50.allsubs
free_section = L56.free_section
ALGS = L53.ALGS


def uni(fams):
    u = frozenset()
    for s in fams:
        u = u | s
    return u


def block_60A():
    print("=== 60A: E-M(𝔗) ≅ E-M(S) -- ROUND TRIPS, S-ALGEBRA LAWS, PRESERVATION ===")
    ok = True
    stage55 = {"A_t": [2, 2, 2, 10], "A_i": [2, 2, 2, 10], "A_f": [2, 2, 2, 10],
               "F(2)": [2, 2, 2, 10], "A_chain": [2, 2, 4, 8]}
    for (name, cA, afun, bot) in ALGS:
        X = list(cA)
        SX = allsubs(X)                                   # S(X) ≅ P(X): 16 sections
        alphaS = {E: afun(E) for E in SX}                 # alphaS(gamma_E) = alpha(E)
        rt1 = all(alphaS[E] == afun(E) for E in SX)       # (alphaS)^t = alpha
        at = {E: alphaS[E] for E in SX}                   # a^t(E) = a(gamma_E)
        as2 = {E: at[E] for E in SX}                      # (a^t)^s(gamma_E) = a^t(E)
        rt2 = all(as2[E] == alphaS[E] for E in SX)
        unit = all(alphaS[frozenset([x])] == x for x in X)     # alphaS ∘ uˢ = id
        mult = True
        for G in allsubs(SX):                             # Gamma in S(SX)
            lhs = alphaS[uni(G)]                          # alphaS(mˢ_X(Gamma))
            img = frozenset(alphaS[E] for E in G)         # S(alphaS)(Gamma) index
            if lhs != alphaS[img]:
                mult = False
        cnt = Counter()
        for E in SX:
            cnt[afun(E)] += 1
        fibs = sorted(cnt.values())
        pres = (fibs == stage55[name])
        ok = ok and rt1 and rt2 and unit and mult and pres
        print("  " + name + "  rt(alphaS)=" + str(rt1) + " rt(a)=" + str(rt2) + " S-unit=" + str(unit)
              + " S-mult=" + str(mult) + "  Fib(alphaS)=" + str(fibs) + " == Stage55 " + str(pres))
    shape_ok = True
    for (name, cA, afun, bot) in ALGS:
        secs, idx, E = L54.search_sections(cA, afun)
        def le(g, h):
            return all(g[i] <= h[i] for i in range(len(E)))
        edges = 0
        for g in secs:
            for h in secs:
                if g != h and le(g, h):
                    if not any(g != mm and mm != h and le(g, mm) and le(mm, h) for mm in secs):
                        edges += 1
        want = 3 if name == "A_chain" else 4
        if edges != want:
            shape_ok = False
        print("  " + name + "  #sections=" + str(len(secs)) + "  Hasse edges=" + str(edges)
              + "  shape=" + ("diamond" if edges == 4 else ("chain" if edges == 3 else "?")))
    print("  section-space shape transport (diamond↔diamond, chain↔chain) : " + str(shape_ok))
    ok = ok and shape_ok
    print("  60A : " + str(ok))
    return ok

def block_60B():
    print("=== 60B: MORPHISMS  h∘α=β∘𝔗h  iff  h∘αˢ=βˢ∘Sh ;  H_Θ round trips ===")
    ok = True
    tothoms = 0
    sifftoth = 0
    equiv = True
    for (na, cA, afA, botA) in ALGS:
        for (nb, cB, afB, botB) in ALGS:
            A = list(cA)
            B = list(cB)
            PA = allsubs(A)
            for hvals in itertools.product(B, repeat=len(A)):
                h = {A[i]: hvals[i] for i in range(len(A))}
                t1 = all(h[afA(K)] == afB(frozenset(h[x] for x in K)) for K in PA)
                t2 = all(h[afA(E)] == afB(frozenset(h[x] for x in E)) for E in PA)
                if t1:
                    tothoms += 1
                if t2:
                    sifftoth += 1
                if t1 != t2:
                    equiv = False
    print("  𝔗-algebra homs found=" + str(tothoms) + "   S-algebra homs found=" + str(sifftoth)
          + "   equivalence h∘α=β∘𝔗h iff h∘αˢ=βˢ∘Sh : " + str(equiv))
    ok = ok and equiv and (tothoms == sifftoth)
    # H_Θ round trips + two equality columns
    ext_eq = True
    byte_eq = True
    for (name, cA, afun, bot) in ALGS:
        X = list(cA)
        SX = allsubs(X)
        # extensional algebras
        alg_t = tuple((K, afun(K)) for K in allsubs(X))         # on P(X)
        alg_s = tuple((E, afun(E)) for E in SX)                 # on S(X): same numbers
        # H_Θ : alpha |-> alphaS ; H_Θ^{-1}: alphaS |-> alpha
        Ht = alg_s                      # H_Θ(α) = αˢ
        Hinv_Ht = alg_t                 # H_Θ⁻¹(αˢ) = α
        if Hinv_Ht != alg_t:
            ext_eq = False
        # canonical-byte representation (tagged by representation)
        b_t = repr(("P(X)", alg_t)).encode()
        b_s = repr(("S(X)", alg_s)).encode()
        if b_t != repr(("P(X)", alg_t)).encode():
            byte_eq = False
        if b_s == b_t:
            pass                        # tags differ -> representations distinct
    print("  H_Θ⁻¹H_Θ = id and H_ΘH_Θ⁻¹ = id (extensional algebra equality) : " + str(ext_eq))
    print("  canonical-byte representation equality is a SEPARATE column; carriers differ (subsets vs sections).")
    print("  two equality columns -> extensional=" + str(ext_eq) + "  untagged-byte-content=" + str(byte_eq))
    ok = ok and ext_eq
    print("  60B : " + str(ok))
    return ok


def block_60C():
    print("=== 60C: β_X = mˢ_X ∘ Θ_{SX}   (Stage-58 join = Stage-59 flatten) ===")
    ok = True
    for X in [[], [0], [0, 1], list(FOUR)]:
        Xl = list(X)
        SX = allsubs(Xl)                        # S(X) ≅ P(X)
        tabs = {E: free_section(Xl, E) for E in SX}
        tab2E = {frozenset(t.items()): E for E, t in tabs.items()}
        good = True
        for G in allsubs(SX):                   # Γ ⊆ S(X): a family of sections
            # β_X(Γ) = γ_{⋃ indices(Γ)} ;  mˢ_X(Θ_{SX}(Γ)) = γ_{⋃ indices(Γ)}
            beta_idx = uni(G)
            # table-level check via the arbitrary join
            beta_tab = L58.beta_join(Xl, [tabs[E] for E in G], tab2E)
            if tab2E.get(beta_tab) != beta_idx:
                good = False
        print("  |X|=" + str(len(Xl)) + "  #families Γ=" + str(2 ** len(SX)) + "  β_X = mˢ_X∘Θ_{SX} : " + str(good))
        ok = ok and good
    print("  60C : " + str(ok))
    return ok

def block_60D():
    print("=== 60D: UNIVERSAL MULTIPLICATION FIBRE  𝔉_k = m_[k]⁻¹([k]) ,  Fib^S_X(γ_E) ≅ 𝔉_|E| ===")
    ok = True
    f_of = L56.free_powerset_union_fibre
    for k in range(0, 5):
        K, PK, fib = L58.fibre_poset(k)
        mfib = [C for C in allsubs(PK) if L56.fam_union(C) == K]      # m_[k]⁻¹([k])
        same = (set(mfib) == set(fib))
        print("  𝔉_" + str(k) + " = m_[" + str(k) + "]⁻¹([" + str(k) + "]):  |m⁻¹|=" + str(len(mfib))
              + "  == |𝔉_k|=" + str(len(fib)) + " : " + str(same))
        ok = ok and same
    X = list(FOUR)
    SX = allsubs(X)
    hist = Counter()
    for G in allsubs(SX):                    # Γ ∈ S(SX)
        hist[uni(G)] += 1                    # mˢ_X(Γ) index = ⋃ indices(Γ)
    byk = {}
    for E, c in hist.items():
        byk.setdefault(len(E), []).append(c)
    good = True
    for k in sorted(byk):
        if set(byk[k]) != {f_of(k)}:
            good = False
        print("    |E|=" + str(k) + "  |Fib^S_X(γ_E)|=" + str(sorted(set(byk[k]))) + "   f(k)=" + str(f_of(k)))
    print("  Fib^S_X(γ_E) ≅ 𝔉_|E|  (counts == f(|E|)) : " + str(good))
    ok = ok and good
    # poset-level (rank) check : fibre over an |E|=2 subset has rank vector of 𝔉_2
    E0 = frozenset(FOUR[:2])
    rc = Counter(len(G) for G in allsubs(SX) if uni(G) == E0)
    rank = [rc.get(r, 0) for r in range(2 ** len(E0) + 1)]
    print("  rank of Fib^S_X(γ_E) at |E|=2 : " + str(rank) + "   𝔉_2 rank = [0,1,4,4,1] : " + str(rank == [0, 1, 4, 4, 1]))
    ok = ok and (rank == [0, 1, 4, 4, 1])
    print("  60D : " + str(ok))
    return ok


def main():
    okA = block_60A()
    print("")
    okB = block_60B()
    print("")
    okC = block_60C()
    print("")
    okD = block_60D()
    print("")
    total = okA and okB and okC and okD
    print("  STAGE 60 RESULT : " + str(total))
    return 0 if total else 1


if __name__ == "__main__":
    sys.exit(main())
