#!/usr/bin/env python3
"""lift_64.py - Stage 64: TERMINAL-TYPE KERNEL / THREE CANONICAL SPINES.

Stage 63 gave ℛ and one spine; Stage 64 refines ℛ from totals to TERMINAL TYPES and
finds THREE canonical re-entry regimes living in the same branching carrier.

    T₀(k,r)=[k=r] ; T_{d+1}(k,s)=Σ_r N(k,r) T_d(r,s) ; h_d(k)=Σ_s T_d(k,s)    64A
    λ(C)={C}, χ(C)={{a}:a∈C}, Ω(C)=P(C)  (ranks 1, r, 2^r)                   64B
    λ: r→1 ; χ: r→r ; Ω: r→2^r  (three iterated type laws)                    64C
    N(r,r) may be >1 ; χ canonical inside the diagonal ; N(r,1)=N(r,2^r)=1    64D
    M₀(k)=k, M_{d+1}(k)=2^(M_d(k)) ; T_d(k,M_d(k))=1 ; Ω^d realises it         64E
    λ, χ, Ω all commute with Cov(b) (relabelling naturality)                   64F

The paraconsistent object held (NOT normalized):
    the FULL TREE (all N(k,r) continuations) stays live, AND it contains three
    canonical spines at once -- λ collapses type, χ preserves type, Ω powersets
    type. None replaces the others.
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


def lam(C):
    """λ(C) = {C}."""
    return frozenset([C])


def chi(C):
    """χ(C) = {{a} : a∈C}."""
    return frozenset(frozenset([a]) for a in C)


def omega(C):
    """Ω(C) = P(C)."""
    return frozenset(allsubs(list(C)))


def lam_n(C, n):
    for _ in range(n):
        C = lam(C)
    return C


def chi_n(C, n):
    for _ in range(n):
        C = chi(C)
    return C


def om_n(C, n):
    for _ in range(n):
        C = omega(C)
    return C


def bmap(b, obj, d):
    """relabel a depth-d nested frozenset object by the ground bijection b."""
    if d == 1:
        return frozenset(b[x] for x in obj)
    return frozenset(bmap(b, x, d - 1) for x in obj)


def _otrack(C, maxsteps=4):
    """Ω rank track: build the powerset while small, else advance symbolically;
    stop before 2^rank becomes unrepresentable."""
    ranks = [len(C)]
    obj = C
    for _ in range(maxsteps):
        if ranks[-1] > 20:
            break
        if len(obj) <= 8:
            obj = omega(obj)
            ranks.append(len(obj))
        else:
            ranks.append(2 ** ranks[-1])
    return ranks


_KMAX1 = 16


def compute_levels(DMAX):
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


_KF = {1: 8, 2: 3, 3: 1, 4: 0}
_Tmemo = {}


def T_dict(d, k):
    if d == 0:
        return {k: 1}
    if k > _KF.get(d, -1):
        return None
    key = (d, k)
    if key in _Tmemo:
        return _Tmemo[key]
    acc = {}
    for r in range(0, 2 ** k + 1):
        c = rank_N(k, r)
        if not c:
            continue
        sub = T_dict(d - 1, r)
        if sub is None:
            _Tmemo[key] = None
            return None
        for s, v in sub.items():
            acc[s] = acc.get(s, 0) + c * v
    _Tmemo[key] = acc
    return acc


def block_64A():
    print("=== 64A: TERMINAL-TYPE KERNEL  T_{d+1}=N·T_d ; h_d=Σ_s T_d(k,s) ; P_d(k;z) ===")
    ok = True
    t1 = True
    for k in range(0, 5):
        Td = T_dict(1, k)
        want = {s: rank_N(k, s) for s in range(0, 2 ** k + 1) if rank_N(k, s)}
        if Td != want:
            t1 = False
    ok = ok and t1
    print("  T₁(k,s) = N(k,s)  (k=0..4) : " + str(t1))
    levels = compute_levels(5)
    marg = True
    for d in [1, 2, 3, 4]:
        for k in range(0, 5):
            Td = T_dict(d, k)
            if Td is None:
                continue
            if sum(Td.values()) != levels[d][k]:
                marg = False
    ok = ok and marg
    print("  marginals Σ_s T_d(k,s) == h_d(k) (Stage-63 levels) : " + str(marg))
    for k in range(0, 5):
        print("    k=" + str(k) + "  h₁=" + brief(levels[1][k]) + "  h₂=" + brief(levels[2][k]))
    # P_0(k;z)=z^k ; P_1(k;z)=R_k(z)
    p0 = all(T_dict(0, k) == {k: 1} for k in range(0, 5))
    p1 = True
    for k in range(0, 5):
        _, Nk = rank_vec(k)
        want = {r: Nk[r] for r in range(len(Nk)) if Nk[r]}
        if T_dict(1, k) != want:
            p1 = False
    ok = ok and p0 and p1
    print("  P₀(k;z)=z^k : " + str(p0) + "   P₁(k;z)=R_k(z) (coeff N(k,r)) : " + str(p1))
    # direct double-sum check T₂(k,s)=Σ_r N(k,r)N(r,s)
    dbl = True
    for k in range(0, 4):
        T2 = T_dict(2, k)
        direct = {}
        for r in range(0, 2 ** k + 1):
            c = rank_N(k, r)
            if not c:
                continue
            for s in range(0, 2 ** r + 1):
                v = rank_N(r, s)
                if v:
                    direct[s] = direct.get(s, 0) + c * v
        if T2 != direct:
            dbl = False
    ok = ok and dbl
    print("  T₂(k,s) == Σ_r N(k,r)N(r,s) (direct double sum, k≤3) : " + str(dbl))
    T2 = T_dict(2, 2)
    parts = ", ".join(str(s) + ":" + str(T2[s]) for s in sorted(T2))
    print("  terminal-type distribution T₂(2,·) (finer than h₂=" + str(levels[2][2]) + "):")
    print("    {" + parts + "}")
    print("  -> R_k is the one-step transition polynomial; ℛ-iterates give the whole depth distribution.")
    print("  64A : " + str(ok))
    return ok


def block_64B():
    print("=== 64B: CANONICAL SELECTORS  λ(C)={C} , χ(C)={{a}:a∈C} , Ω(C)=P(C) ; ranks 1, r, 2^r ===")
    ok = True
    for k in range(0, 5):
        fib, Nk = rank_vec(k)
        b1 = Nk[1]
        bmax = Nk[2 ** k] if 2 ** k < len(Nk) else 0
        agree = (rank_N(k, 1) == 1) and (rank_N(k, 2 ** k) == 1)
        ok = ok and (b1 == 1) and (bmax == 1) and agree
        print("  k=" + str(k) + "  N(k,1)=" + str(b1) + " (closed " + str(rank_N(k, 1)) + ")"
              + "   N(k,2^k)=" + str(bmax) + " (closed " + str(rank_N(k, 2 ** k)) + ")")
    sel = True; tested = 0
    for k in range(0, 3):
        fib, Nk = rank_vec(k)
        for C in fib:
            r = len(C)
            PC = frozenset(allsubs(list(C)))
            for F, want in ((lam(C), 1), (chi(C), r), (omega(C), 2 ** r)):
                if len(F) != want or F > PC or fam_union(F) != C:
                    sel = False
            tested += 1
    ok = ok and sel
    print("  every C (k=0,1,2): λ,χ,Ω ∈ Fib₂(C) with ranks 1, |C|, 2^|C| : " + str(sel)
          + "   (" + str(tested) + " nodes)")
    print("  -> ONE NODE, THREE canonical re-entry dynamics; for r>1 they are concretely distinct.")
    print("  64B : " + str(ok))
    return ok


def block_64C():
    print("=== 64C: THREE ITERATED TYPE LAWS  λ:r→1 , χ:r→r , Ω:r→2^r ===")
    ok = True
    lam_ok = True; chi_ok = True; om_ok = True; om_build = True
    for k in range(0, 4):
        fib, Nk = rank_vec(k)
        for C in fib:
            r = len(C)
            tl = [len(lam_n(C, n)) for n in range(0, 5)]
            if tl != [r, 1, 1, 1, 1]:
                lam_ok = False
            tc = [len(chi_n(C, n)) for n in range(0, 5)]
            if tc != [r, r, r, r, r]:
                chi_ok = False
            ranks = _otrack(C, 4)
            if ranks != Mseq(r, len(ranks) - 1):
                om_ok = False
    ok = ok and lam_ok and chi_ok and om_ok
    print("  λ track [r,1,1,1,1] on every node (k≤3) : " + str(lam_ok))
    print("  χ track [r,r,r,r,r] on every node (k≤3) : " + str(chi_ok))
    print("  Ω track [r, 2^r, 2^(2^r), …] on every node (k≤3) : " + str(om_ok))
    fib, Nk = rank_vec(2)
    C0 = max(fib, key=lambda F: (len(F), sorted(map(sorted, F))))
    r0 = len(C0)
    om_track = _otrack(C0, 4)
    print("  sample (r=" + str(r0) + "):  λ-track " + str([r0, 1, 1, 1, 1])
          + "   χ-track " + str([r0] * 5) + "   Ω-track " + str(om_track))
    print("  χ: type fixed AND representation winds ; Ω: type winds by powerset AND representation winds.")
    print("  64C : " + str(ok))
    return ok


def block_64D():
    print("=== 64D: CANONICAL vs MULTIPLICITY  diagonal multiplicity N(r,r) ; χ canonical inside it ===")
    ok = True
    nrr = {}
    for r in range(0, 5):
        fib, Nk = rank_vec(r)
        nranks = sum(1 for C in fib if len(C) == r)
        nrr[r] = rank_N(r, r)
        if rank_N(r, r) != nranks or rank_N(r, 1) != 1 or rank_N(r, 2 ** r) != 1:
            ok = False
    print("  N(r,r) = " + ", ".join(str(nrr[r]) for r in range(0, 5)) + "   (diagonal multiplicity)")
    print("  N(r,1)=1 and N(r,2^r)=1 for every r  (λ and Ω strata are singleton strata)")
    # χ is the unique irredundant max-rank cover
    fib, Nk = rank_vec(2)
    C0 = [C for C in fib if len(C) == 2][0]
    cand = [F for F in allsubs(allsubs(list(C0)))
            if fam_union(F) == C0 and len(F) == 2 and all(fam_union(F - {S}) != C0 for S in F)]
    uniq = (len(cand) == 1 and cand[0] == chi(C0))
    ok = ok and uniq
    print("  r=2: N(2,2)=4 rank-2 families, yet over the rank-2 C exactly "
          + str(len(cand)) + " irredundant rank-2 cover == χ(C) : " + str(uniq))
    print("  diagonal transition multiplicity = N(r,r) ; canonical diagonal selector = χ. The spine is one")
    print("  distinguished point inside the diagonal stratum -- canonical branch ≠ only branch.")
    print("  64D : " + str(ok))
    return ok


_Mt = {}


def Mseq(k, d):
    m = k; out = [m]
    for _ in range(d):
        m = 2 ** m; out.append(m)
    return out


def Tterm(d, k, s):
    if d == 0:
        return 1 if k == s else 0
    if d == 1:
        return rank_N(k, s)
    key = (d, k, s)
    if key in _Mt:
        return _Mt[key]
    tot = 0
    for r in range(0, 2 ** k + 1):
        c = rank_N(k, r)
        if c:
            tot += c * Tterm(d - 1, r, s)
    _Mt[key] = tot
    return tot


def block_64E():
    print("=== 64E: MAXIMAL BRANCH  M_{d+1}(k)=2^(M_d(k)) ; T_d(k,M_d(k))=1 ; Ω^d realises it ===")
    ok = True
    for k in range(0, 3):
        seq = Mseq(k, 4)
        print("  k=" + str(k) + "  M(·) = " + " -> ".join(brief(x) for x in seq))
        for d in [1, 2, 3]:
            got = Tterm(d, k, seq[d])
            good = (got == 1)
            ok = ok and good
            print("       T_" + str(d) + "(" + str(k) + ", M_" + str(d) + "=" + brief(seq[d])
                  + ") = " + str(got) + " : " + str(good))
    # Ω^d realises the max terminal rank on a built example
    fib, Nk = rank_vec(2)
    C0 = [C for C in fib if len(C) == 2][0]
    ranks = _otrack(C0, 3)
    seq2 = Mseq(len(C0), len(ranks) - 1)
    realize = all(ranks[i] == seq2[i] for i in range(len(ranks)))
    ok = ok and realize
    print("  Ω^d on a rank-" + str(len(C0)) + " C realises M_d : " + str(ranks) + " == " + str(seq2[:len(ranks)])
          + " : " + str(realize))
    print("  -> exactly ONE globally maximal path C→P(C)→P(P(C))→… ; V_{n+1}=P(V_n) appears internally as")
    print("     the unique top branch of the complete decomposition tree. Held: canonical AND not the whole tree.")
    print("  64E : " + str(ok))
    return ok


def block_64F():
    print("=== 64F: NATURALITY OF ALL THREE SELECTORS  Cov(b)∘selⁿ = selⁿ∘Cov(b) ===")
    ok = True

    def run(sel_n, depth_extra_max, kk):
        natural = True; tested = 0
        fib, Nk = rank_vec(kk)
        perms = list(itertools.permutations(range(kk)))
        for C in fib:
            for p in perms:
                b = {i: p[i] for i in range(kk)}
                for n in range(0, depth_extra_max + 1):
                    try:
                        lhs = bmap(b, sel_n(C, n), 2 + n)
                        rhs = sel_n(bmap(b, C, 2), n)
                    except MemoryError:
                        continue
                    if lhs != rhs:
                        natural = False
                    tested += 1
        return natural, tested

    for kk in range(0, 4):
        nl, tl = run(lam_n, 3, kk)
        nc, tc = run(chi_n, 3, kk)
        ok = ok and nl and nc
        print("  k=" + str(kk) + "  λ natural (n≤3) : " + str(nl) + "   χ natural (n≤3) : " + str(nc)
              + "   (" + str(tl) + " / " + str(tc) + " checks)")
    # Ω naturality (n≤2, bounded build), k=0..2
    om_nat = True; om_tested = 0
    for kk in range(0, 3):
        fib, Nk = rank_vec(kk)
        perms = list(itertools.permutations(range(kk)))
        for C in fib:
            for p in perms:
                b = {i: p[i] for i in range(kk)}
                for n in range(0, 3):
                    built = om_n(C, n)
                    if len(built) > 2048:
                        continue
                    if bmap(b, built, 2 + n) != om_n(bmap(b, C, 2), n):
                        om_nat = False
                    om_tested += 1
    ok = ok and om_nat
    print("  Ω natural (n≤2, k≤2) : " + str(om_nat) + "   (" + str(om_tested) + " checks)")
    print("  -> all three spines are label-independent; the branching fibre type AND each chosen spine is natural.")
    print("  64F : " + str(ok))
    return ok


def main():
    okA = block_64A(); print("")
    okB = block_64B(); print("")
    okC = block_64C(); print("")
    okD = block_64D(); print("")
    okE = block_64E(); print("")
    okF = block_64F(); print("")
    total = okA and okB and okC and okD and okE and okF
    print("  STAGE 64 RESULT : " + str(total))
    return 0 if total else 1


if __name__ == "__main__":
    sys.exit(main())
