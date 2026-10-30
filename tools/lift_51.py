#!/usr/bin/env python3
"""lift_51.py - Stage 51: E-M OBJECTS RE-ENTER MODULO ISOMORPHISM.

Stage 49 put the E-M algebras into E0 as CONTENT; Stage 50 measured
A_t ≅ A_i ≅ A_f ≅ F(2) as E-M algebras.  So the object being re-entered now has
TWO readings, and Stage 51 keeps BOTH live at once:

    Eraw = encoded E-M objects                 (presentation identity)
    Eiso = Eraw / ≅EM                           (categorical identity)
    T(Eraw)=P(Eraw)     T(Eiso)=P(Eiso)     q : Eraw -> Eiso,  P(q)(S)={ [A] | A in S }

Decisive witness:  K_EM = {A_t,A_i,A_f}
    |K_EM|raw = 3          (three carrier-fixed evaluation structures)
    |P(q)(K_EM)| = 1       (one E-M isomorphism class)
Not a collapse -- the next ≈U/≈G/=EM-style distinction, now at categorical level.
Non-free witness K+ = {A_t,A_i,A_f,A_chain}: raw 4, but |P(q)(K+)| = 2.

 51A  four invariants measured SEPARATELY:  raw cardinality / U-classes /
      G-image classes / E-M isomorphism classes.
 51B  the transported square  h(alpha_A K) = alpha_B(P(h) K)  for EVERY E-M
      isomorphism among {A_t,A_i,A_f,F(2)}.  Same named K={T,F} stays
      presentation-sensitive (T/B/F); the transported K commutes exactly.
 51C  Eval_S : P(X)->P(X),  Eval_S(K) = { alpha_A(K) | A in S };  the set laws;
      the higher map  Eval : P(E_X) -> (P(X) -> P(X))  -- evaluation structures
      re-enter as ONE higher value carrying MANY ways of evaluating ONE lower value.
"""
import sys, os, itertools
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lift_41 import member, canon_set
from lift_45 import alpha, FOUR, BOT
import lift_43 as L43
import lift_49 as L49
import lift_50 as L50
import lift_reentry as R

ORD3 = ["t", "i", "f"]
word = L43.word
allsubs = L50.allsubs
FOUR_CARRIER = L49.FOUR_CARRIER   # encoded carrier WORD
CARRIER = FOUR                    # carrier ELEMENT list for iso/subset work

# ----------------------------------------------------------- carrier functions
CRISP = {o: (lambda K, o=o: alpha(o, sorted(K))) for o in ORD3}
CHAINORD = {"N": 0, "T": 1, "F": 2, "B": 3}

def alpha_chain(K):
    """4-chain join: sup in the order N<T<F<B; N is the chain bottom."""
    K = list(K)
    return "N" if not K else max(K, key=lambda x: CHAINORD[x])

def graph_of(afun):
    """encode an algebra map by its full graph { K |-> alpha(K) : K subset FOUR }."""
    items = [member(L49.pairw(word(K), afun(sorted(K)))) for K in L49.four_subsets()]
    return R.FSPLIT + "".join(items) + R.FFUSE

def encobj(o):
    """(name, carrier, afun) -> (carrier, counit-encoding) = an ALG object."""
    return (FOUR_CARRIER, graph_of(o[2]))

def encw(o):
    return L49.alg_word(*encobj(o))

Eraw = [
    ("A_t",     CARRIER, CRISP["t"]),
    ("A_i",     CARRIER, CRISP["i"]),
    ("A_f",     CARRIER, CRISP["f"]),
    ("A_chain", CARRIER, alpha_chain),
]
NAME2OBJ = {o[0]: o for o in Eraw}

# ------------------------------------------------------------ iso machinery
def isos(f1, c1, f2, c2):
    """every E-M isomorphism h:(c1,f1)->(c2,f2):  h(f1 K) = f2({h x})."""
    out = []
    if len(c1) != len(c2):
        return out
    subs = allsubs(c1)
    for perm in itertools.permutations(c2):
        phi = dict(zip(c1, perm))
        if all(phi[f1(K)] == f2(sorted({phi[x] for x in K})) for K in subs):
            out.append(phi)
    return out

def iso_pair(o1, o2):
    return len(isos(o1[2], o1[1], o2[2], o2[1])) > 0

def iso_classes(objs):
    """partition objs by E-M isomorphism (union-find)."""
    n = len(objs)
    par = list(range(n))
    def find(i):
        while par[i] != i:
            par[i] = par[par[i]]
            i = par[i]
        return i
    for i in range(n):
        for j in range(i + 1, n):
            if iso_pair(objs[i], objs[j]):
                par[find(i)] = find(j)
    cls = {}
    for i in range(n):
        cls.setdefault(find(i), []).append(objs[i][0])
    return [sorted(v) for v in cls.values()]

def Genc(o):
    return L49.G_enc(encobj(o))

# --------------------------------------------------- 51A  four invariants, apart
def invariants(label, objs):
    raws = {encw(o) for o in objs}
    Ucls = {encobj(o)[0] for o in objs}
    Gcls = {Genc(o) for o in objs}
    icls = iso_classes(objs)
    print("   " + label)
    print("        |raw| = " + str(len(raws)) + "   |U-classes| = " + str(len(Ucls)) +
          "   |G-image classes| = " + str(len(Gcls)) +
          "   |\u2245EM-classes| = " + str(len(icls)))
    for cl in sorted(icls):
        print("        E-M iso class : " + " \u2261 ".join(cl))
    return (len(raws), len(Ucls), len(Gcls), len(icls))

def reentry_A():
    print("=== 51A: FOUR INVARIANTS MEASURED SEPARATELY ===")
    KEM = [NAME2OBJ[n] for n in ("A_t", "A_i", "A_f")]
    Kpl = [NAME2OBJ[n] for n in ("A_t", "A_i", "A_f", "A_chain")]
    a = invariants("K_EM = {A_t, A_i, A_f}", KEM)
    b = invariants("K+   = {A_t, A_i, A_f, A_chain}", Kpl)
    # the quotient map q : Eraw -> Eiso, by iso-class representative (canonical min)
    classes = iso_classes(Eraw)
    rep = {}
    for cl in classes:
        r = min(cl)
        for nm in cl:
            rep[nm] = r
    def q(nm):
        return rep[nm]
    def Pq(names):
        return sorted({q(n) for n in names})
    KEMn = ["A_t", "A_i", "A_f"]
    Kpln = ["A_t", "A_i", "A_f", "A_chain"]
    # well-definedness: q(A)=q(B) iff A ≅ B
    wd = all((q(x[0]) == q(y[0])) == iso_pair(x, y) for x in Eraw for y in Eraw)
    print("   Eraw = " + str(len(Eraw)) + " encoded ALG objects ;  Eiso = Eraw/\u2245EM, classes = " +
          str(len(classes)))
    print("   q well-defined (q(A)=q(B) iff A \u2245 B) : " + str(wd))
    print("   |P(q)(K_EM)| = " + str(len(Pq(KEMn))) + "   " + str(Pq(KEMn)))
    print("   |P(q)(K+)|   = " + str(len(Pq(Kpln))) + "   " + str(Pq(Kpln)))
    print("   => THREE carrier-fixed evaluation structures AND ONE categorical iso class --")
    print("      not a collapse: the Stage-49 triple is presentation identity, Eiso is")
    print("      categorical identity, and both stay live.")
    print("   => K+ adds A_chain: G-image does NOT separate it (|G-cl|=1) but \u2245EM does")
    print("      (|\u2245EM-cl|=2) -- the non-free witness is invisible to G, visible to iso.")
    print()
    ok = (a == (3, 1, 1, 1)) and (b == (4, 1, 1, 2)) and len(Pq(KEMn)) == 1 \
        and len(Pq(Kpln)) == 2 and wd
    return ok

# --------------------------------------- 51B  every iso + the transported square
def reentry_B():
    print("=== 51B: EVERY E-M ISOMORPHISM AND THE TRANSPORTED SQUARE ===")
    a, b = "a", "b"
    Fc = [frozenset(), frozenset({a}), frozenset({b}), frozenset({a, b})]
    def union_family(KK):
        r = frozenset()
        for K in KK:
            r = r | K
        return r
    F2 = ("F(2)", Fc, union_family)
    objs = [NAME2OBJ["A_t"], NAME2OBJ["A_i"], NAME2OBJ["A_f"], F2]
    total = 0
    allsq = True
    found = {}
    for o1 in objs:
        for o2 in objs:
            hs = isos(o1[2], o1[1], o2[2], o2[1])
            found[(o1[0], o2[0])] = hs
            for phi in hs:
                total += 1
                if not all(phi[o1[2](K)] == o2[2](sorted({phi[x] for x in K}))
                           for K in allsubs(o1[1])):
                    allsq = False
            print("   iso " + o1[0] + " -> " + o2[0] + " : count = " + str(len(hs)))
    print("   total E-M isomorphisms among {A_t,A_i,A_f,F(2)} : " + str(total))
    print("   transported square h(alpha_A K) = alpha_B(P(h) K) holds for EVERY iso : " + str(allsq))
    # same named K is presentation-sensitive
    K0 = ["T", "F"]
    vals = {o: alpha(o, K0) for o in ORD3}
    print("   SAME named K={T,F}: alpha_t=" + str(vals["t"]) + "  alpha_i=" + str(vals["i"]) +
          "  alpha_f=" + str(vals["f"]) + "   (presentation-sensitive: 3 distinct values)")
    # transported K commutes exactly along a chosen iso A_t -> A_i
    h = found[("A_t", "A_i")][0]
    K0s = frozenset(K0)
    transported = sorted(h[x] for x in K0s)
    lhs = h[CRISP["t"](K0s)]
    rhs = CRISP["i"](sorted({h[x] for x in K0s}))
    print("   TRANSPORT K0 along h : A_t -> A_i : h(K0) = " + str(transported) +
          " (a DIFFERENT named subset)")
    print("   h(alpha_t(K0)) = " + str(lhs) + "  =  alpha_i(P(h)(K0)) = " + str(rhs) +
          "   (commutes exactly)")
    print("   => different in fixed coordinates AND the same structure up to E-M isomorphism.")
    print()
    return allsq and total > 0 and len(set(vals.values())) == 3

# ------------------------------------ 51C  Eval_S : the evaluation higher object
def Eval(afuns, K):
    """Eval_S(K) = { alpha_A(K) | A in S },  S a set of eval structures on one carrier X."""
    return frozenset(f(K) for f in afuns)

def reentry_C():
    print("=== 51C: Eval_S -- EVALUATION STRUCTURES RE-ENTER ===")
    t, i, f, ch = CRISP["t"], CRISP["i"], CRISP["f"], alpha_chain
    Ks = allsubs(FOUR)
    print("   Eval_S(K) = { alpha_A(K) | A in S }   (S a set of eval structures on carrier X)")
    print("   Eval_{K_EM}({T,F}) = " + str(sorted(Eval([t, i, f], frozenset({"T", "F"})))))
    # the three set laws of the algebra-family argument
    S, Rr = [t, i], [f, ch]
    empt = all(Eval([], K) == frozenset() for K in Ks)
    unio = all(Eval(S + Rr, K) == (Eval(S, K) | Eval(Rr, K)) for K in Ks)
    sing = all(Eval([g], K) == frozenset({g(K)}) for g in [t, i, f, ch] for K in Ks)
    print("   Eval_\u2205(K) = \u2205                            : " + str(empt))
    print("   Eval_{S\u222aR}(K) = Eval_S(K) \u222a Eval_R(K)    : " + str(unio))
    print("   Eval_{{A}}(K) = { alpha_A(K) }            : " + str(sing))
    # the higher map  Eval : P(E_X) -> (P(X) -> P(X))
    KEM = [t, i, f]
    Kplus = [t, i, f, ch]
    imgEM = max(len(Eval(KEM, K)) for K in Ks)
    imgPl = max(len(Eval(Kplus, K)) for K in Ks)
    print("   Eval : P(E_X) -> (P(X) -> P(X))   -- ONE higher value carries MANY ways")
    print("   of evaluating ONE lower-level re-entered value (more than words-in-a-SET).")
    print("   crisp triple  max_K |Eval_{K_EM}(K)| = " + str(imgEM))
    print("   with A_chain  max_K |Eval_{K+}(K)|   = " + str(imgPl))
    print("   A_chain shares underlying carrier X = FOUR with the crisp triple, so its")
    print("   evaluation profile lives in the SAME higher object P(E_X).")
    print()
    return empt and unio and sing

def main():
    print("=== Stage 51: E-M OBJECTS RE-ENTER MODULO ISOMORPHISM ===")
    a = reentry_A()
    b = reentry_B()
    c = reentry_C()
    ok = a and b and c
    print("STAGE51: re-entering the E-M objects keeps presentation identity AND categorical")
    print("   identity at once: K_EM is THREE carrier-fixed evaluation structures AND ONE")
    print("   E-M isomorphism class; the transported square commutes for EVERY iso, so")
    print("   evaluation survives re-entry modulo isomorphism.  Eval : P(E_X)->(P(X)->P(X))")
    print("   makes P(Eraw) operational: one higher value carries many ways of evaluating one")
    print("   lower-level re-entered value.  Eiso = Eraw/\u2245EM is the categorical reading.")
    print("   51A invariants " + str(a) + " | 51B transported square " + str(b) +
          " | 51C Eval " + str(c) + " => " + str(ok))
    print("   NEXT (Stage 52): E-M objects re-entering themselves (internal re-entry).")

if __name__ == "__main__":
    main()
