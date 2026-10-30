#!/usr/bin/env python3
"""lift_52.py - Stage 52: THE E-M ALGEBRA RE-ENTERS ITSELF.

No external powerset this time: the algebra's OWN evaluation leaves the powerset
carrier and its OWN unit re-enters it,

    rho_A := u_{UA} o U(eps_A)          P(UA) --eps_A=alpha_A--> UA --u--> P(UA)
    rho_A(K) = { alpha_A(K) }.

Stage 47's triangle identity gives   U eps_A o u_UA = id_UA.  Stage 52 takes the
REVERSE composite   u_UA o U eps_A = rho_A,  which need not be the identity -- it
is automatically a SPLIT IDEMPOTENT:

    rho^2 = u alpha u alpha = u (alpha u) alpha = u alpha = rho.

So: evaluate a re-entered distinction, re-inscribe the result as a singleton
distinction, and further iteration stops.

  52A internal re-entry exhaustively, every K subset of each carrier, incl. empty
  52B the fixed locus  Fix(rho_A) = im(u_UA) = { {a} | a in UA }  (a singleton spine)
  52C TRANSPORT:  P(h) o rho_A = rho_B o P(h)  for EVERY E-M hom (Stage-49 homs)
  52D the crisp witness rho_t({T,F})={T}, rho_i({T,F})={B}, rho_f({T,F})={F};
      A_chain computed independently; the alpha-u loop; the Stage-38/39 recurrence

The sharp paraconsistent witness:

    same underlying subset {T,F}   AND   different internal re-entry fixed points.

Recurrence (structural, NOT an identification of the maps):
    Stage 38/39   pi o eta = id,  eta o pi = C,  C^2 = C
    Stage 52      alpha o u = id,  u o alpha = rho,  rho^2 = rho
"""
import sys, os, itertools
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lift_41 import member, canon_set
from lift_45 import alpha, FOUR, BOT
import lift_43 as L43
import lift_49 as L49
import lift_50 as L50
import lift_51 as L51
import lift_reentry as R

ORD3 = ["t", "i", "f"]
word = L43.word
allsubs = L50.allsubs
CARRIER = FOUR

CRISP = {o: (lambda K, o=o: alpha(o, sorted(K))) for o in ORD3}
CHAINORD = {"N": 0, "T": 1, "F": 2, "B": 3}

def alpha_chain(K):
    K = list(K)
    return "N" if not K else max(K, key=lambda x: CHAINORD[x])

# F(2) = (P({a,b}), union) : carrier is the 4 subsets of {a,b}
a, b = "a", "b"
Fc = [frozenset(), frozenset({a}), frozenset({b}), frozenset({a, b})]

def union_family(KK):
    r = frozenset()
    for K in KK:
        r = r | K
    return r

# (name, carrier elements, algebra map, bottom of the order)
ALGS = [
    ("A_t",     CARRIER, CRISP["t"],    BOT["t"]),   # bottom F
    ("A_i",     CARRIER, CRISP["i"],    BOT["i"]),   # bottom N
    ("A_f",     CARRIER, CRISP["f"],    BOT["f"]),   # bottom T
    ("F(2)",    Fc,      union_family,  frozenset()),
    ("A_chain", CARRIER, alpha_chain,   "N"),
]

# ------------------------------------------------------------ internal re-entry
def rho(afun, K):
    """rho_A(K) = u(alpha_A(K)) = { alpha_A(K) }   : P(UA) -> P(UA)."""
    return frozenset({afun(K)})

def rho_graph(f):
    """encode the map rho on the FOUR carrier by its graph (all K subset FOUR)."""
    items = [member(L49.pairw(word(K), word(rho(f, K)))) for K in allsubs(CARRIER)]
    return R.FSPLIT + "".join(items) + R.FFUSE

# ------------------------------------------------ 52A  internal re-entry, all K
def internal_reentry():
    print("=== 52A: INTERNAL RE-ENTRY  rho_A = u o U(alpha_A),  rho_A(K) = { alpha_A(K) } ===")
    ok = True
    for (name, c, f, bot) in ALGS:
        Ks = allsubs(c)
        idem = all(rho(f, rho(f, K)) == rho(f, K) for K in Ks)
        a_eval = all(f(rho(f, K)) == f(K) for K in Ks)          # alpha(rho K) = alpha K
        r_single = all(rho(f, frozenset({x})) == frozenset({x}) for x in c)   # rho({a})={a}
        card1 = all(len(rho(f, K)) == 1 for K in Ks)
        empty = (rho(f, frozenset()) == frozenset({bot}))       # rho(empty)={bottom}
        ok &= idem and a_eval and r_single and card1 and empty
        print("   " + name + ":  rho^2=rho " + str(idem) + "  alpha(rho K)=alpha K " + str(a_eval) +
              "  rho({x})={x} " + str(r_single) + "  |rho(K)|=1 " + str(card1) +
              "  rho(empty)={bottom} " + str(empty))
    print("   => rho soaks the carrier back to a singleton; further iteration stops (split idempotent).")
    print("   => empty content does NOT stay empty: rho(empty) = {bottom_A}  (∅ != {bottom}).")
    print()
    return ok

# ------------------------------------------------------- 52B  the fixed locus
def fixed_locus():
    print("=== 52B: THE FIXED LOCUS  Fix(rho_A) = im(u_UA) = { {a} | a in UA } ===")
    ok = True
    for (name, c, f, bot) in ALGS:
        Ks = allsubs(c)
        fix = {K for K in Ks if rho(f, K) == K}
        singles = {frozenset({x}) for x in c}
        eq = (fix == singles)
        empty_in = (frozenset() in fix)
        ok &= eq and (not empty_in)
        print("   " + name + ":  |Fix(rho)| = " + str(len(fix)) +
              "  |singletons| = " + str(len(singles)) + "  equal " + str(eq) +
              "  empty fixed? " + str(empty_in))
    print("   => rho_A retracts the whole free cover P(UA) onto the singleton spine")
    print("      A --u--> P(A),  im(u) = { {a} | a in UA }.")
    print("   => ∅ is NOT fixed: rho(∅) = {bottom} != ∅; the empty family and the singleton")
    print("      {bottom} are DISTINCT re-entered objects.")
    print()
    return ok

# ---------------------------------------------- 52C  transport along every E-M hom
def transport():
    print("=== 52C: TRANSPORT  P(h) o rho_A = rho_B o P(h)  for EVERY E-M hom ===")
    print("   shape:  P(h)({alpha_A K}) = {h(alpha_A K)} = {alpha_B(P(h) K)} = rho_B(P(h) K)")
    ok = True
    total = 0
    maps = list(itertools.product(CARRIER, repeat=4))
    idx = {x: i for i, x in enumerate(CARRIER)}

    def is_hom(h, o, p):
        for K in allsubs(CARRIER):
            if h[idx[alpha(o, sorted(K))]] != alpha(p, sorted({h[idx[x]] for x in K})):
                return False
        return True

    for o in ORD3:
        for p in ORD3:
            hs = [h for h in maps if is_hom(h, o, p)]
            good = True
            for h in hs:
                for K in allsubs(CARRIER):
                    lhs = frozenset({h[idx[CRISP[o](K)]]})
                    rhs = rho(CRISP[p], frozenset({h[idx[x]] for x in K}))
                    if lhs != rhs:
                        good = False
            total += len(hs)
            ok &= good
            print("   hom " + o + "->" + p + " : " + str(len(hs)) + " maps, transport holds on all K : " + str(good))
    # isomorphisms out to F(2) as well
    for (name, c, f, bot) in [ALGS[0], ALGS[1], ALGS[2]]:
        hs = L51.isos(f, c, union_family, Fc)
        good = True
        for h in hs:
            for K in allsubs(c):
                lhs = frozenset({h[f(K)]})
                rhs = rho(union_family, frozenset({h[x] for x in K}))
                if lhs != rhs:
                    good = False
        total += len(hs)
        ok &= good
        print("   iso " + name + "->F(2) : " + str(len(hs)) + " maps, transport holds on all K : " + str(good))
    print("   total E-M homs checked : " + str(total) + "  ;  transported internal re-entry : " + str(ok))
    print("   => different presentations have different rho in fixed coordinates AND")
    print("      rho transports naturally along E-M morphisms (Stage-51 square lifted to rho).")
    print()
    return ok

# ---------------------------------------------- 52D  witness, chain, recurrence
def witness():
    print("=== 52D: THE CRISP WITNESS, A_chain (computed independently), AND THE LOOP ===")
    K = frozenset({"T", "F"})
    for o in ORD3:
        print("   rho_" + o + "({T,F}) = " + str(sorted(rho(CRISP[o], K))))
    print("   A_chain computed independently (NOT inferred from free/non-free status):")
    print("        rho_chain({T,F}) = " + str(sorted(rho(alpha_chain, K))))
    print("        rho_chain(empty) = " + str(sorted(rho(alpha_chain, frozenset()))))
    print("   => SAME underlying subset {T,F} AND different internal re-entry fixed points.")
    # the loop / triangle identity pair
    au = all(f(frozenset({x})) == x for (_, c, f, _) in ALGS for x in c)     # alpha o u = id
    print("   alpha o u = id (unit law)   : " + str(au))
    print("   u o alpha = rho            : by definition of rho")
    print("   rho^2 = rho                : verified in 52A for every K")
    print("   rho as content: rho-ENC bytes = " + str(len(rho_graph(CRISP["t"]))))
    print("   recurrence (structural, NOT an identification of the maps):")
    print("        Stage 38/39   pi o eta = id ,  eta o pi = C ,  C^2 = C")
    print("        Stage 52      alpha o u = id ,  u o alpha = rho ,  rho^2 = rho")
    print()
    return au

def main():
    print("=== Stage 52: THE E-M ALGEBRA RE-ENTERS ITSELF ===")
    a = internal_reentry()
    b = fixed_locus()
    c = transport()
    d = witness()
    ok = a and b and c and d
    print("STAGE52: rho_A = u_{UA} o U(eps_A), rho_A(K)={alpha_A(K)}, is the E-M algebra's OWN")
    print("   internal re-entry -- a split idempotent (rho^2=rho) whose fixed locus is the")
    print("   singleton spine im(u_UA)={ {a} | a in UA }; it transports naturally along every")
    print("   E-M hom; and the same re-entered subset {T,F} returns to {T}/{B}/{F} under the")
    print("   three evaluation structures -- same content AND different fixed points.")
    print("   52A re-entry " + str(a) + " | 52B fixed locus " + str(b) +
          " | 52C transport " + str(c) + " | 52D witness " + str(d) + " => " + str(ok))
    print("   NEXT (Stage 53): the fixed-point spine as an E-M object / the idempotent's image.")

if __name__ == "__main__":
    main()
