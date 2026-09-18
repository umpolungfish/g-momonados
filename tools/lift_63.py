#!/usr/bin/env python3
"""lift_63.py - Stage 63: THE FIBRE TRANSFER OPERATOR / STATIONARY χ-SPINE.

Stage 62 found the recursion; Stage 63 turns it into an OPERATOR and separates
exhaustive branching from canonical re-entry.

    (ℛh)(k) := Σ_r N(k,r) h(r)          N(k,r) = #{C∈𝔉_k : |C|=r}      63A
    h₀=1 , h₁=ℛh₀=f=|𝔉_k| , h₂=ℛ²h₀=|𝔇_k| , h_{d+1}=ℛh_d
    τ(C)=|C| ; τ∘χ=τ (rank fixed) ; χⁿ changes representation         63B
    Path_d(k) ; |Path_d(k)|=h_d(k) ; Spine_d(C)⊂Path_d(k)              63C
    χ = canonical re-entry (unique max-rank cover) ≠ only branch       63D
    Cov(b)∘χⁿ = χⁿ∘Cov(b)  (naturality under relabelling)              63E

NOTATION FIX (Stage 62 -> 63): since 𝔉_r is NOT an inherited E-M subalgebra
(Stage 61D), the type-safe statement is Fib₂(C) ≅ 𝔉_|C| as sets/posets/fibre
objects with matching rank geometry -- not ≅_EM unless a structure is transported.

The paraconsistent object held (NOT normalized):
    the full branching tower ℛ retains every continuation  AND  χ selects one
    coherent continuation; TYPE IS FIXED while REPRESENTATION KEEPS WINDING.
"""
import sys, os, itertools
from collections import Counter
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
try:
    sys.set_int_max_str_digits(200000)
except Exception:
    pass
from lift_45 import alpha, FOUR, BOT
import lift_50 as L50
import lift_53 as L53
import lift_56 as L56
import lift_58 as L58

allsubs = L50.allsubs
fam_union = L56.fam_union
f_of = L56.free_powerset_union_fibre
fibre_poset = L58.fibre_poset
rank_N = L58.rank_N

_fc = {}


def fc(r):
    if r not in _fc:
        _fc[r] = f_of(r)
    return _fc[r]


def brief(n):
    s = str(n)
    return s if len(s) <= 40 else (s[:24] + "...(+" + str(len(s)) + " digits)")


def rank_vec(k):
    K, PK, fib = fibre_poset(k)
    rc = Counter(len(C) for C in fib)
    return fib, [rc.get(r, 0) for r in range(len(PK) + 1)]


def chi(C):
    """comultiplication χ(C) = {{a} : a∈C}."""
    return frozenset(frozenset([a]) for a in C)


def chi_n(C, n):
    for _ in range(n):
        C = chi(C)
    return C


_KMAX1 = 16


def compute_levels(DMAX):
    """h_1(k)=f(k) closed form; h_{d+1}(k)=Σ_r N(k,r) h_d(r) where tractable."""
    levels = {}
    levels[1] = {k: fc(k) for k in range(0, _KMAX1 + 1)}
    for d in range(2, DMAX + 1):
        prev = levels[d - 1]
        mx = max(prev)
        cur = {}
        for k in range(0, _KMAX1 + 1):
            if 2 ** k > mx:
                continue
            tot = 0
            for r in range(0, 2 ** k + 1):
                c = rank_N(k, r)
                if c:
                    tot += c * prev[r]
            cur[k] = tot
        levels[d] = cur
    return levels


def block_63A():
    print("=== 63A: TRANSFER OPERATOR  (ℛh)(k)=Σ_r N(k,r) h(r) ; h₁=f=|𝔉_k| , h₂=𝔇_k , h_{d+1}=ℛh_d ===")
    ok = True
    for k in range(0, 5):
        fib, Nk = rank_vec(k)
        closed = [rank_N(k, r) for r in range(len(Nk))]
        match = (closed == Nk)
        ok = ok and match
        print("  k=" + str(k) + "  N(k,·) measured == rank_N closed form : " + str(match))
    levels = compute_levels(5)
    h1 = levels[1]; h2 = levels[2]
    v1 = True
    for k in range(0, 5):
        fib, Nk = rank_vec(k)
        if h1[k] != fc(k) or sum(Nk) != fc(k) or len(fib) != fc(k):
            v1 = False
    ok = ok and v1
    print("  h₀ = 1 everywhere;  h₁(k)=ℛh₀(k)=Σ_r N(k,r)=f(k)=|𝔉_k| : " + str(v1))
    print("    h₁ : " + ", ".join(str(h1[k]) for k in range(0, 5)) + " , ...   (f(k))")
    v2 = True
    for k in range(0, 5):
        fib, Nk = rank_vec(k)
        Dk = sum(Nk[r] * fc(r) for r in range(len(Nk)))
        if h2[k] != Dk:
            v2 = False
    ok = ok and v2
    print("    h₂ : " + ", ".join(brief(h2[k]) for k in range(0, 5)) + "   (== |𝔇_k| : " + str(v2) + ")")
    for d in [3, 4, 5]:
        lv = levels[d]
        print("    h" + str(d) + " : " + "   ".join("k=" + str(k) + " -> " + brief(lv[k]) for k in sorted(lv)))
    print("  law  h_{d+1}(k)=Σ_r N(k,r) h_d(r)  ;  h_d(k) = # depth-d decomposition paths from type k.")
    print("  63A : " + str(ok))
    return ok


def block_63B():
    print("=== 63B: STATIONARY χ TYPE  τ(C)=|C| ; τ∘χ=τ (rank fixed) ; χⁿ winds representation ===")
    fixed = True; winds = True; rank_all = True
    for k in range(0, 4):
        K, PK, fib = fibre_poset(k)
        for C in fib:
            if len(chi(C)) != len(C):
                fixed = False
            for n in range(0, 5):
                if len(chi_n(C, n)) != len(C):
                    rank_all = False
            if len(C) >= 1:
                if chi(C) == C:
                    winds = False
                for n in range(1, 5):
                    if chi_n(C, n) == chi_n(C, n - 1):
                        winds = False
    ok = fixed and winds and rank_all
    # sample track
    K, PK, fib = fibre_poset(2)
    C0 = max(fib, key=lambda F: (len(F), sorted(map(sorted, F))))
    track = [len(chi_n(C0, n)) for n in range(0, 5)]
    print("  τ∘χ = τ (rank fixed at every level n=0..4) : " + str(fixed and rank_all))
    print("  χⁿ(C) ≠ χⁿ⁻¹(C) as encoded objects (representation winds) : " + str(winds))
    print("  sample C (rank " + str(len(C0)) + "):  rank track τ(χⁿC) = " + str(track)
          + "   (constant " + str(len(set(track)) == 1) + ")")
    print("  -> TYPE IS FIXED  AND  REPRESENTATION KEEPS WINDING.  Do NOT call χ a fixed point: qualified τ∘χ=τ.")
    print("  63B : " + str(ok))
    return ok


def enumerate_paths(k, d):
    """materialize Path_d(k) = {(C₁..C_d) : C₁∈𝔉_k, C_{j+1}∈𝔉_{|C_j|}}."""
    fib = fibre_poset(k)[2]
    if d == 1:
        return [(C,) for C in fib]
    out = []
    for C in fib:
        for tail in enumerate_paths(len(C), d - 1):
            out.append((C,) + tail)
    return out


def valid_path(k, path):
    if not path:
        return False
    if path[0] not in set(fibre_poset(k)[2]):
        return False
    for j in range(1, len(path)):
        prev = path[j - 1]
        if not (path[j] <= frozenset(allsubs(list(prev))) and fam_union(path[j]) == prev):
            return False
    return True


def block_63C():
    print("=== 63C: PATH CARRIER  |Path_d(k)|=ℛ^d(1)(k) ; canonical Spine_d(C) sits inside the full carrier ===")
    levels = compute_levels(5)
    ok = True
    plan = [(0, 1), (0, 2), (0, 3), (0, 4), (1, 1), (1, 2), (1, 3), (2, 1), (2, 2), (3, 1), (4, 1)]
    for (k, d) in plan:
        n = len(enumerate_paths(k, d))
        want = levels[d][k]
        good = (n == want)
        ok = ok and good
        print("  |Path_" + str(d) + "(" + str(k) + ")| = " + str(n) + "   ℛ^" + str(d) + "(1)(" + str(k)
              + ") = " + brief(want) + " : " + str(good))
    # spine inside the carrier
    spine_ok = True; rank_ok = True; k = 3
    K, PK, fib = fibre_poset(k)
    for C in fib:
        for d in range(1, 5):
            sp = tuple(chi_n(C, j) for j in range(0, d))
            if not valid_path(k, sp):
                spine_ok = False
            if len(set(len(x) for x in sp)) != 1:
                rank_ok = False
    ok = ok and spine_ok and rank_ok
    print("  Spine_d(C) = (C,χC,χ²C,…,χ^{d-1}C) ∈ Path_d(k) for all C (k=3, d≤4) : " + str(spine_ok))
    print("  τ constant along every spine : " + str(rank_ok))
    print("  63C : " + str(ok))
    return ok


def block_63D():
    print("=== 63D: DISTINGUISHED DIAGONAL  N(r,r) may be >1 ; χ picks ONE canonical r→r branch ===")
    ok = True
    for r in range(0, 5):
        Nrr = rank_N(r, r)
        K, PK, fibr = fibre_poset(r)
        nranks = sum(1 for C in fibr if len(C) == r)
        agree = (Nrr == nranks)
        ok = ok and agree
        print("  r=" + str(r) + "  N(r,r) = " + str(Nrr) + "  (=#rank-r members of 𝔉_r : " + str(agree) + ")")
    # concrete: r=2 has 4 rank-2 covers; χ picks the unique irredundant maximal-rank one
    K, PK, fib2 = fibre_poset(2)
    rank2 = [C for C in fib2 if len(C) == 2]
    x = chi(rank2[0]) if rank2 else frozenset()
    print("  r=2 : 𝔉_2 has N(2,2)=" + str(len(rank2)) + " rank-2 families (many r→r branches)")
    # unique irredundant max-rank cover of a rank-2 C
    C0 = [C for C in fib2 if len(C) == 2][0]
    cand = [F for F in allsubs(allsubs(list(C0))) if fam_union(F) == C0 and len(F) == 2
            and all(fam_union(F - {S}) != C0 for S in F)]
    print("      over the rank-2 C=" + str(sorted(sorted(s) for s in C0)) + ": "
          + str(len(cand)) + " irredundant rank-2 cover -> the singleton cover χ(C) : "
          + str(len(cand) == 1 and cand[0] == chi(C0)))
    ok = ok and (len(cand) == 1 and cand[0] == chi(C0))
    print("  ℛ retains EVERY continuation (N(r,r) branches) ; χ selects ONE coherent continuation.")
    print("  63D : " + str(ok))
    return ok


def bmap(b, obj, d):
    """relabel a depth-d nested frozenset object by the ground bijection b."""
    if d == 1:
        return frozenset(b[x] for x in obj)
    return frozenset(bmap(b, x, d - 1) for x in obj)


def block_63E():
    print("=== 63E: NATURALITY  bmap(b, χⁿC, 2+n) = χⁿ(bmap(b, C, 2))  for all permutations, several depths ===")
    ok = True
    for k in range(0, 4):
        K, PK, fib = fibre_poset(k)
        perms = list(itertools.permutations(range(k)))
        natural = True; tested = 0
        for C in fib:
            for p in perms:
                b = {i: p[i] for i in range(k)}
                for n in range(0, 4):
                    lhs = bmap(b, chi_n(C, n), 2 + n)
                    rhs = chi_n(bmap(b, C, 2), n)
                    if lhs != rhs:
                        natural = False
                    tested += 1
        ok = ok and natural
        print("  k=" + str(k) + "  #perms=" + str(len(perms)) + "  naturality χⁿ∘Cov = Cov∘χⁿ (n≤3) : "
              + str(natural) + "   (" + str(tested) + " checks)")
    # k=4 : identity + a transposition, sampled C
    k = 4
    K, PK, fib = fibre_poset(k)
    natural4 = True; tested4 = 0
    for p in [tuple(range(4)), (1, 0, 2, 3)]:
        b = {i: p[i] for i in range(4)}
        for C in fib[:400]:
            for n in range(0, 4):
                if bmap(b, chi_n(C, n), 2 + n) != chi_n(bmap(b, C, 2), n):
                    natural4 = False
                tested4 += 1
    ok = ok and natural4
    print("  k=4  identity + transposition, 400 sampled C, n≤3 : " + str(natural4)
          + "   (" + str(tested4) + " checks)")
    print("  -> the whole branching fibre type is relabelling-invariant AND the χ-spine is natural.")
    print("  63E : " + str(ok))
    return ok


def main():
    okA = block_63A(); print("")
    okB = block_63B(); print("")
    okC = block_63C(); print("")
    okD = block_63D(); print("")
    okE = block_63E(); print("")
    total = okA and okB and okC and okD and okE
    print("  STAGE 63 RESULT : " + str(total))
    return 0 if total else 1


if __name__ == "__main__":
    sys.exit(main())
