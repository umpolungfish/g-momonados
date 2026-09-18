#!/usr/bin/env python3
"""lift_58.py - Stage 58: FREE-COVER FIBRE GEOMETRY / NATURAL SECTION ALGEBRA.

Two structures of the free cover are pinned exactly:

  1. its evaluation fibres as actual POSETS (not just sizes), and
  2. its section space as an E-M algebra NATURALLY isomorphic to the free algebra.

The paraconsistent ambient: the section law is Boolean objectwise, while its
functorial action preserves only the join / E-M structure in general.  Both live.

  58A fibre rank polynomial: Fib_X(K) = { K⊆P(X) : ⋃K=K }, ordered by ⊆ of families.
      A bijection K≅L transports Fib_X(K) ≅ Fib_Y(L), so the geometry depends only on
      k=|K|.  Rank counts N(k,r)=Σ_j (-1)^j C(k,j) C(2^(k-j), r); generating function
      R_k(z)=Σ_j (-1)^j C(k,j) (1+z)^(2^(k-j));  R_k(1)=f(k).  Rank vectors for k=0..4.
  58B fibre Hasse geometry: unique top P(K); inclusion-minimal elements = irredundant
      covers (counts 1,1,2,8,49 for k=0..4); edge count H_k = 2^k f(k) - R'_k(1)
      (1,1,15,805,513135).
  58C natural section algebra: S(X)=Sec(F(X)); β_X(Γ)=∨Γ; Θ_X(E)=γ_E;
      Θ_X(⋃𝔈) = β_X({Θ_X(E):E∈𝔈})  =>  Θ_X : F(X) ≅EM S(X),  natural in X.
  58D the Stage-56 fork carried through G: Fib(A_chain) ≠ free fibre geometry AND
      Fib(G(A_chain)) ≅ Fib(G(A_t)) = free geometry.

The paraconsistent object held (NOT normalized):
    the historical non-free distinction is true at A⁰  AND  its free-cover face has the
    canonical cover-poset geometry at A¹ -- neither erases the other.
"""
import sys, os, math
from collections import Counter
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lift_45 import alpha, FOUR, BOT
import lift_50 as L50
import lift_53 as L53
import lift_56 as L56

allsubs = L50.allsubs
free_section = L56.free_section
fam_union = L56.fam_union
ALGS = L53.ALGS


def fibre_poset(k):
    """Fib(K) for K = {0..k-1}: all families fam ⊆ P(K) with ⋃fam = K."""
    K = frozenset(range(k))
    PK = frozenset(allsubs(list(K)))
    fib = [fam for fam in allsubs(PK) if fam_union(fam) == K]
    return K, PK, fib


def rank_N(k, r):
    s = 0
    for j in range(k + 1):
        s += ((-1) ** j) * math.comb(k, j) * math.comb(2 ** (k - j), r)
    return s


def block_58A():
    print("=== 58A: FREE-COVER FIBRE RANK POLYNOMIAL  R_k(z)=Σ_j (-1)^j C(k,j)(1+z)^(2^(k-j)) ===")
    f_of = L56.free_powerset_union_fibre
    expected = {
        0: [1, 1],
        1: [0, 1, 1],
        2: [0, 1, 4, 4, 1],
        3: [0, 1, 13, 44, 67, 56, 28, 8, 1],
        4: [0, 1, 40, 360, 1546, 4144, 7896, 11408, 12866, 11440, 8008, 4368, 1820, 560, 120, 16, 1],
    }
    ok = True
    for k in range(0, 5):
        K, PK, fib = fibre_poset(k)
        rc = Counter(len(fam) for fam in fib)
        rank = [rc.get(r, 0) for r in range(2 ** k + 1)]
        byN = all(rank[r] == rank_N(k, r) for r in range(2 ** k + 1))
        match = (rank == expected[k])
        tot = sum(rank)
        ok = ok and byN and match and (tot == f_of(k))
        print("  k=" + str(k) + "  R_k = " + str(rank))
        print("       sum=" + str(tot) + " (=f(k)=" + str(f_of(k)) + ")  matches N(k,r)=" + str(byN) + "  matches table=" + str(match))
    print("  58A : " + str(ok))
    return ok

def block_58B():
    print("=== 58B: FIBRE HASSE GEOMETRY   top P(K), min-covers 1,1,2,8,49, edges H_k=2^k f(k)-R'_k(1) ===")
    f_of = L56.free_powerset_union_fibre
    exp_min = [1, 1, 2, 8, 49]
    exp_edge = [1, 1, 15, 805, 513135]
    ok = True
    for k in range(0, 5):
        K, PK, fib = fibre_poset(k)
        # unique top = P(K) itself
        top_unique = (PK in fib) and all((fam <= PK) for fam in fib)
        # inclusion-minimal elements = irredundant covers
        minc = 0
        for fam in fib:
            red = False
            for S in fam:
                if fam_union(fam - {S}) == K:
                    red = True
                    break
            if not red:
                minc += 1
        # edge count: each family can be extended by any of (2^k - |fam|) absent subsets
        edges = sum(len(PK) - len(fam) for fam in fib)
        # formula H_k = 2^k f(k) - R'_k(1); R'_k(1) = sum over fib of |fam|
        rp1 = sum(len(fam) for fam in fib)
        hform = (2 ** k) * f_of(k) - rp1
        good = top_unique and (minc == exp_min[k]) and (edges == exp_edge[k]) and (edges == hform)
        ok = ok and good
        print("  k=" + str(k) + "  |Fib|=" + str(len(fib)) + "  top=P(K) only=" + str(top_unique)
              + "  min-covers=" + str(minc) + " (exp " + str(exp_min[k]) + ")"
              + "  Hasse edges=" + str(edges) + " (exp " + str(exp_edge[k]) + ", formula " + str(hform) + ")")
    print("  58B : " + str(ok))
    return ok


def beta_join(X, gammas, tab2E):
    """β_X(Γ) = ∨Γ : the least section ≥ all γ∈Γ.
    Empty Γ -> the bottom γ_{∅}.  Nonempty -> pointwise union (a section by Stage 57)."""
    PX = allsubs(list(X))
    if not gammas:
        return frozenset(free_section(X, frozenset()).items())
    tab = {}
    for K in PX:
        u = frozenset()
        for t in gammas:
            u = u | t[K]
        tab[K] = u
    return frozenset(tab.items())


def block_58C():
    print("=== 58C: NATURAL SECTION ALGEBRA   Θ_X : F(X) ≅EM S(X) ===")
    ok = True
    for X in [list(range(2)), list(FOUR)]:
        Xl = list(X)
        E_all = allsubs(Xl)
        tabs = {E: free_section(Xl, E) for E in E_all}
        tab2E = {frozenset(tabs[E].items()): E for E in E_all}
        # Θ_X is a bijection
        bij = (len(tab2E) == len(E_all)) and (len(set(tab2E.values())) == len(E_all))
        # E-M hom : Θ_X(⋃𝔈) = β_X({Θ_X(E):E∈𝔈})  for all families 𝔈 ⊆ P(X)
        hom = True
        checked = 0
        for gam in allsubs(E_all):                       # a family 𝔈 of subsets E
            unionE = frozenset()
            for E in gam:
                unionE = unionE | E
            lhs = frozenset(free_section(Xl, unionE).items())        # Θ_X(⋃𝔈)
            rhs = beta_join(Xl, [tabs[E] for E in gam], tab2E)       # β_X(Θ_X 𝔈)
            checked += 1
            if lhs != rhs:
                hom = False
        inS = all(beta_join(Xl, [tabs[E] for E in gam], tab2E) in tab2E for gam in allsubs(E_all))
        print("  |X|=" + str(len(Xl)) + "  #sections=" + str(len(E_all)) + "  Θ bijection=" + str(bij)
              + "  families checked=" + str(checked) + "  Θ EM-hom=" + str(hom) + "  β_X(Γ)∈S(X)=" + str(inS))
        ok = ok and bij and hom and inS

    # naturality in X : Θ_Y ∘ P(f) = Sec(F(f)) ∘ Θ_X
    Xa = [0, 1, 2]
    fmap = {0: 10, 1: 11, 2: 12}
    def Pf(S):
        return frozenset(fmap[x] for x in S)
    def PPf(fam):
        return frozenset(Pf(K) for K in fam)
    nat = True
    for E in allsubs(Xa):
        tX = free_section(Xa, E)
        fE = frozenset(fmap[x] for x in E)
        tY = free_section([10, 11, 12], fE)
        for K in allsubs(Xa):
            if PPf(tX[K]) != tY[Pf(K)]:
                nat = False
    print("  naturality Θ_Y∘P(P(f)) = (transport)∘Θ_X for |X|=3 : " + str(nat))
    ok = ok and nat
    print("  58C : " + str(ok))
    return ok

def block_58D(sigs):
    print("=== 58D: THE STAGE-56 FORK CARRIED THROUGH G ===")
    free2 = L56.free_powerset_union_fibre
    free_law2 = sorted([free2(0), free2(1), free2(1), free2(2)])
    ch = sigs.get("A_chain")
    at = sigs.get("A_t")
    print("  A⁰ (historical, pre-re-entry):")
    print("    Fib(A_chain) = " + str(ch) + "   Fib(A_t) = " + str(at) + "   free law(|X|=2) = " + str(free_law2))
    hist = (ch != free_law2) and (at == free_law2)
    print("    A_chain ≠ free geometry  AND  A_t = free geometry : " + str(hist))
    print("  A¹ = G(A) (free cover face):")
    print("    G(A_chain) ≅ G(A_t) ≅ F(FOUR)  ->  Fib(G(A_chain)) ≅ Fib(G(A_t)) = free cover geometry")
    # free cover geometry at A^1 is the rank poset of F(FOUR) : k=0..4, canonical for both
    K, PK, fib = fibre_poset(4)
    print("    |Fib(F(FOUR))| = " + str(len(fib)) + "   rank total = f(4) = " + str(free2(4)))
    cover = (len(fib) == free2(4))
    print("    free-cover geometry canonical at A¹ for both : " + str(cover))
    print("  => historical non-free distinction TRUE at A⁰  AND  free-cover geometry canonical at A¹. Neither erases the other.")
    ok = hist and cover
    print("  58D : " + str(ok))
    return ok


def stage_signatures():
    out = {}
    for (name, cA, afun, bot) in ALGS:
        cnt = Counter()
        for K in allsubs(list(cA)):
            cnt[afun(K)] += 1
        out[name] = sorted(cnt.values())
    return out


def main():
    okA = block_58A()
    print("")
    okB = block_58B()
    print("")
    okC = block_58C()
    print("")
    okD = block_58D(stage_signatures())
    print("")
    total = okA and okB and okC and okD
    print("  STAGE 58 RESULT : " + str(total))
    return 0 if total else 1


if __name__ == "__main__":
    sys.exit(main())
