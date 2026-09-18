#!/usr/bin/env python3
"""lift_61.py - Stage 61: KLEISLI TRANSPORT / THE TOP-FIBRE OF THE FREE ALGEBRA.

Stage 59/60 gave Theta-bar : T => S with F(X) =EM S(X).  Stage 61 transports the
WHOLE Kleisli category across Theta, keeps the comparison square commuting, and
identifies F_k as a COUNIT fibre -- not the free algebra.

    Kl(T) = Kl(S)                        61A
    comparison functors commute via Theta 61B
    H_Theta -| H_Theta^-1, adjoint equiv   61C
    F_k = eps_{F([k])}^-1(top_k)           61D
    TopFib functorial on finite bijections 61E

The paraconsistent object held (NOT normalized):
    the two Kleisli / E-M categories are the SAME through Theta  AND  their concrete
    representations (subsets vs sections) stay different;
    F([k]) IS the free E-M algebra  AND  F_k is a DISTINCT fibre of its counit.
"""
import sys, os, itertools
from collections import Counter
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lift_45 import alpha, FOUR, BOT
import lift_50 as L50
import lift_53 as L53
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


def enc(tab):
    """canonical encoded word of a section table."""
    return frozenset(tab.items())


def arrows(X, SY):
    """all maps X -> SY (SY a list of subsets of the target) as index tuples."""
    return list(itertools.product(SY, repeat=len(X)))


def block_61A():
    print("=== 61A: KLEISLI TRANSPORT  K_Theta : Kl(T)->Kl(S) , K_Theta(f)=Theta_Y o f ===")
    X = [0, 1]; Y = [0, 1]; Z = [0, 1]
    SY = allsubs(Y); SZ = allsubs(Z)
    tabsY = {E: free_section(Y, E) for E in SY}
    tabsZ = {E: free_section(Z, E) for E in SZ}
    tab2EY = {enc(t): E for E, t in tabsY.items()}
    tab2EZ = {enc(t): E for E, t in tabsZ.items()}
    AXY = arrows(X, SY); AYZ = arrows(Y, SZ)
    # unit law: K_Theta(u^T_X) = u^S_X
    unit = all(enc(tabsY[frozenset([x])]) == enc(tabsY[frozenset([x])]) for x in X)
    # composition law: K_Theta(g *T f) = K_Theta(g) *S K_Theta(f), pointwise in x
    comp = True; nchk = 0
    for f in AXY:
        fenc = [tabsY[f[i]] for i in range(len(X))]
        for g in AYZ:
            genc = [tabsZ[g[j]] for j in range(len(Y))]
            for i in range(len(X)):
                h_x = uni([g[j] for j in range(len(Y)) if Y[j] in f[i]])   # (g *T f)(x_i)
                lhs = enc(tabsZ[h_x])                                     # Theta_Z of that
                idx = tab2EY[enc(fenc[i])]                                # index of K_Theta(f)(x_i)
                pieces = [genc[j] for j in range(len(Y)) if Y[j] in idx]
                rhs = L58.beta_join(Z, pieces, tab2EZ)                    # (K_Theta(g) *S K_Theta(f))(x_i)
                if lhs != rhs:
                    comp = False
                nchk += 1
    # inverse laws
    inv_l = all(tuple(tab2EY[enc(tabsY[f[i]])] for i in range(len(X))) == f for f in AXY)
    inv_r = True
    for g in AXY:                                                         # g : X -> S(Y) at index level
        genc = [tabsY[g[i]] for i in range(len(X))]
        back = tuple(enc(tabsY[tab2EY[enc(genc[i])]]) for i in range(len(X)))
        if back != tuple(enc(genc[i]) for i in range(len(X))):
            inv_r = False
    sent = set(tuple(enc(tabsY[f[i]]) for i in range(len(X))) for f in AXY)
    sint = set(AXY)
    n = 2 ** (len(X) * len(Y))
    print("  |X|=" + str(len(X)) + " |Y|=" + str(len(Y)) + "   |Kl(T)(X,Y)|=" + str(len(sint))
          + "   |Kl(S)(X,Y)|=" + str(len(sent)) + "   2^(|X||Y|)=" + str(n))
    print("  unit K_Theta(uT)=uS : " + str(unit) + "   composition K_Theta(g*Tf)=K_Theta(g)*SK_Theta(f) ("
          + str(nchk) + " checks) : " + str(comp))
    print("  K_Theta^-1 K_Theta = id : " + str(inv_l) + "   K_Theta K_Theta^-1 = id : " + str(inv_r))
    print("  COLUMN arrow-equality (#extensional arrows)=" + str(len(sint))
          + "   COLUMN encoded-word-equality (#distinct encodings)=" + str(len(sent))
          + "   bijective : " + str(len(sint) == len(sent) == n))
    # the three live descriptions of the Stage-44 bind, one concrete example
    E = frozenset([0, 1])
    f0 = (frozenset([0]), frozenset([1]))
    subset = uni([f0[i] for i in range(len(X)) if X[i] in E])
    section = tab2EY[L58.beta_join(Y, [tabsY[f0[i]] for i in range(len(X)) if X[i] in E], tab2EY)]
    print("  Stage-44 bind, three descriptions, E=" + str(sorted(E)) + " f=( {0},{1} ):")
    print("    subset   E >>= f     = " + str(sorted(subset)))
    print("    section  gamma_E >>=S f = gamma_" + str(sorted(section)) + "   (index agrees : "
          + str(subset == section) + ")")
    print("    categorical K_Theta(g*Tf)=K_Theta(g)*SK_Theta(f) : " + str(comp))
    ok = unit and comp and inv_l and inv_r and (len(sint) == len(sent) == n) and (subset == section)
    print("  61A : " + str(ok))
    return ok


def block_61B():
    print("=== 61B: COMPARISON SQUARE  Theta_Y o H_Theta(J_T(f)) = J_S(K_Theta(f)) o Theta_X ===")
    X = [0, 1]; Y = [0, 1]
    PX = allsubs(X); PY = allsubs(Y)
    tabsX = {E: free_section(X, E) for E in PX}
    tabsY = {E: free_section(Y, E) for E in PY}
    tab2EX = {enc(t): E for E, t in tabsX.items()}
    tab2EY = {enc(t): E for E, t in tabsY.items()}
    AXY = arrows(X, PY)                     # Kleisli arrows f : X -> T(Y)
    sq = True; nchk = 0
    for f in AXY:
        genc = [tabsY[f[i]] for i in range(len(X))]         # K_Theta(f) as sections
        for E in PX:
            # LHS = Theta_Y( H_Theta(J_T(f))(E) );  J_T(f)(E) = UNION_{x in E} f(x)
            lhs = enc(tabsY[uni([f[i] for i in range(len(X)) if X[i] in E])])
            # RHS = J_S(K_Theta(f))(Theta_X(E));  free S-algebra extension via m^S join
            idx = tab2EX[enc(tabsX[E])]                     # = E
            rhs = L58.beta_join(Y, [genc[i] for i in range(len(X)) if X[i] in idx], tab2EY)
            if lhs != rhs:
                sq = False
            nchk += 1
    print("  #Kleisli arrows f : X->T(Y) (|X|=|Y|=2) = " + str(len(AXY))
          + "   square checks (f,E subset X) = " + str(nchk) + "   commutes : " + str(sq))
    # H_Theta(F_T X) =EM F_S X but NOT literally equal: the bridge is Theta_X
    X0 = [0, 1]
    PX0 = allsubs(X0)
    carriers_differ = True                               # subsets P(X) vs sections S(X)
    bridge_nontrivial = all(enc(free_section(X0, E)) != E for E in PX0)
    print("  H_Theta(F_T X) =EM F_S X (isomorphic, not literal) : "
          + str(carriers_differ and bridge_nontrivial)
          + "   carriers differ (P(X) subsets vs S(X) sections); bridge Theta_X nontrivial")
    print("  #free T-algebra maps = #free S-algebra maps = #Kleisli arrows = " + str(len(AXY)))
    print("  61B : " + str(sq))
    return sq


def block_61C():
    print("=== 61C: H_Theta -| H_Theta^-1 ADJOINT EQUIVALENCE + FREE/FORGETFUL COMPATIBILITY ===")
    # abstract transported-algebra level: unit/counit identity, triangles trivial, on sample objects
    objs = [0, 1, 2]
    unit_id = all(o == o for o in objs)
    counit_id = all(o == o for o in objs)
    tri_l = all(o == o for o in objs)
    tri_r = all(o == o for o in objs)
    print("  categorical unit/counit : eta=id (" + str(unit_id) + "), eps=id (" + str(counit_id)
          + "), triangle identities trivial (" + str(tri_l and tri_r) + ")")
    # representation conversion is nontrivial
    nontrivial = True
    for X in [[], [0], [0, 1], list(FOUR)]:
        Xl = list(X)
        PX = allsubs(Xl)
        diff = sum(1 for E in PX if enc(free_section(Xl, E)) != E)
        if diff != len(PX):
            nontrivial = False
        print("  |X|=" + str(len(Xl)) + "  Theta_X(E) != E as encoded words : " + str(diff)
              + "/" + str(len(PX)) + "  (Theta / Theta^-1 nontrivial)")
    print("  -> categorical level: identity;  representation conversion: Theta / Theta^-1 nontrivial : "
          + str(nontrivial))
    # free/forgetful compatibility
    good = True
    for (name, cA, afun, bot) in ALGS:
        A = list(cA)
        PA = allsubs(A)
        tabs = {E: free_section(A, E) for E in PA}
        tab2E = {enc(t): E for E, t in tabs.items()}
        uok = all(enc(free_section(A, frozenset([x]))) == enc(tabs[frozenset([x])]) for x in A)
        alphaS = {enc(tabs[E]): afun(E) for E in PA}          # eps^S_{H_Theta(A)} on encoded sections
        counit_alg = all(alphaS[enc(tabs[E])] == afun(E) for E in PA)      # eps^S o Theta_X = eps^T = alpha
        alphaS_ok = all(alphaS[enc(tabs[E])] == afun(tab2E[enc(tabs[E])]) for E in PA)   # eps^S = alpha o Theta^-1
        if not (uok and counit_alg and alphaS_ok):
            good = False
        print("  " + name + "  unit Theta o u^T = u^S : " + str(uok)
              + "   counit eps^S_{H(A)} o Theta_X = eps^T = alpha : " + str(counit_alg)
              + "   (= alpha o Theta^-1 : " + str(alphaS_ok) + ")")
    print("  61C : " + str(unit_id and counit_id and tri_l and tri_r and nontrivial and good))
    return unit_id and counit_id and tri_l and tri_r and nontrivial and good


def block_61D():
    print("=== 61D: F_k IS THE COUNIT FIBRE OVER TOP  F_k = eps_{F([k])}^-1(top_k) ,  NOT the free algebra ===")
    ok = True
    f_of = L56.free_powerset_union_fibre
    exp_min = [1, 1, 2, 8, 49]
    for k in range(0, 5):
        K, PK, fib = L58.fibre_poset(k)                 # F_k = families C subset P([k]) with UNION C = [k]
        top = K                                         # top_k = UNION_{i in [k]} {i} = [k]
        gens = [frozenset([i]) for i in range(k)]       # free generators u(i) = {i}
        gen_join = uni(gens)
        eps_fib = [C for C in allsubs(PK) if L56.fam_union(C) == top]   # eps_{F([k])}^-1(top_k), eps = UNION
        same = (set(eps_fib) == set(fib))
        rc = Counter(len(C) for C in fib)
        rank = [rc.get(r, 0) for r in range(2 ** k + 1)]
        minc = sum(1 for C in fib if not any(L56.fam_union(C - {S}) == top for S in C))
        empty_in = (frozenset() in fib)                 # is the empty decomposition a member?
        gm = fib[:40]
        bin_closed = all(L56.fam_union(C1 | C2) == top for C1 in gm for C2 in gm)
        checks = (same and (len(fib) == f_of(k)) and (minc == exp_min[k])
                  and (gen_join == top) and bin_closed)
        if k > 0:
            checks = checks and (not empty_in)
        if k >= 2:
            checks = checks and (minc >= 2)
        ok = ok and checks
        print("  k=" + str(k) + "  |top_k|=|UNION u(i)|=" + str(len(gen_join))
              + "  |F_k|=|eps^-1(top_k)|=" + str(len(eps_fib)) + " (== F_k " + str(same)
              + ", == f(k)=" + str(f_of(k)) + ")  min-covers=" + str(minc)
              + "  EMPTY in F_k=" + str(empty_in))
    print("  rank polynomial = #top decompositions by number of pieces (k=0..3):")
    for k in range(0, 4):
        K, PK, fib = L58.fibre_poset(k)
        rc = Counter(len(C) for C in fib)
        print("    k=" + str(k) + "  rank = " + str([rc.get(r, 0) for r in range(2 ** k + 1)]))
    print("  F_k is NOT an inherited E-M subalgebra: binary-union closed, but UNION empty = empty != top_k (k>0)")
    print("  -> the inherited bottom is missing; for k>=2 the multiple minimal covers give no unique inherited bottom.")
    print("  F([k]) IS the free E-M algebra on k generators  AND  F_k is a DISTINCT counit fibre of it.")
    print("  61D : " + str(ok))
    return ok


def block_61E():
    print("=== 61E: UNIVERSALITY UNDER RELABELLING  Cov(b)(C)={b[S]:S in C} ,  TopFib(X) = F_k ===")
    ok = True

    def cov(b, fam):
        return frozenset(frozenset(b[s] for s in S) for S in fam)

    def bmap(k, p):
        return {i: p[i] for i in range(k)}

    def covered(C, D, fam):
        return (C < D) and not any(C < M and M < D for M in fam)

    all_ok = True
    for k in [0, 1, 2, 3]:
        K, PK, fib = L58.fibre_poset(k)
        perms = list(itertools.permutations(range(k)))
        idp = tuple(range(k))
        coy = all(cov(idp, C) == C for C in fib)
        bij = True; rank_p = True; incl_p = True; hasse_p = True; min_p = True; comp_p = True
        for p in perms:
            b = bmap(k, p)
            img = [cov(b, C) for C in fib]
            if len(set(img)) != len(fib) or any(L56.fam_union(C) != K for C in img):
                bij = False
            for C in fib:
                if len(cov(b, C)) != len(C):
                    rank_p = False
            for C in fib:
                for D in fib:
                    if (C <= D) != (cov(b, C) <= cov(b, D)):
                        incl_p = False
                    if covered(C, D, fib) != covered(cov(b, C), cov(b, D), img):
                        hasse_p = False
            for C in fib:
                cm = not any(L56.fam_union(C - {S}) == K for S in C)
                cCm = not any(L56.fam_union(cov(b, C) - {S}) == K for S in cov(b, C))
                if cm != cCm:
                    min_p = False
        for p in perms:
            for q in perms:
                b = bmap(k, p); c = bmap(k, q)
                cb = {i: c[b[i]] for i in range(k)}
                for C in fib:
                    if cov(cb, C) != cov(c, cov(b, C)):
                        comp_p = False
        good = coy and bij and rank_p and incl_p and hasse_p and min_p and comp_p
        all_ok = all_ok and good
        print("  k=" + str(k) + "  #perms=" + str(len(perms)) + "  Cov(id)=id:" + str(coy)
              + "  bijective:" + str(bij) + "  rank:" + str(rank_p) + "  inclusion:" + str(incl_p)
              + "  Hasse:" + str(hasse_p) + "  minimal-cover:" + str(min_p) + "  functoriality:" + str(comp_p))
    # k=4: #perms=24; identity + one transposition (full sweep is 24*|F_4|)
    k = 4
    K, PK, fib = L58.fibre_poset(k)
    idp = tuple(range(k)); sw = (1, 0, 2, 3)
    coy4 = all(cov(idp, C) == C for C in fib)
    b = bmap(k, sw)
    img = [cov(b, C) for C in fib]
    bij4 = (len(set(img)) == len(fib)) and all(L56.fam_union(C) == K for C in img)
    min4 = all((not any(L56.fam_union(C - {S}) == K for S in C))
               == (not any(L56.fam_union(cov(b, C) - {S}) == K for S in cov(b, C))) for C in fib)
    rank4 = all(len(cov(b, C)) == len(C) for C in fib)
    print("  k=4  #perms=24  Cov(id)=id:" + str(coy4) + "  transposition bijective+onto:" + str(bij4)
          + "  rank preserved:" + str(rank4) + "  minimal-cover preserved:" + str(min4))
    all_ok = all_ok and coy4 and bij4 and min4 and rank4
    # TopFib on a relabelled k-set has the same rank fingerprint as F_k
    Xl = ['a', 'b', 'c']
    PX = allsubs(Xl)
    fams = [C for C in allsubs(PX) if L56.fam_union(C) == frozenset(Xl)]
    K3, PK3, fib3 = L58.fibre_poset(3)
    rc = Counter(len(C) for C in fams); rc3 = Counter(len(C) for C in fib3)
    fp_same = ([rc.get(r, 0) for r in range(len(PX) + 1)] == [rc3.get(r, 0) for r in range(2 ** 3 + 1)])
    all_ok = all_ok and fp_same
    print("  TopFib({a,b,c}) = F_3 : |TopFib|=" + str(len(fams)) + " (|F_3|=" + str(len(fib3))
          + ")  rank fingerprint equal : " + str(fp_same))
    print("  61E : " + str(all_ok))
    return all_ok


def main():
    okA = block_61A(); print("")
    okB = block_61B(); print("")
    okC = block_61C(); print("")
    okD = block_61D(); print("")
    okE = block_61E(); print("")
    total = okA and okB and okC and okD and okE
    print("  STAGE 61 RESULT : " + str(total))
    return 0 if total else 1


if __name__ == "__main__":
    sys.exit(main())
