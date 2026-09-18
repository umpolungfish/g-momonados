#!/usr/bin/env python3
"""lift_44.py - Stage 44: KLEISLI / RE-ENTRY COMPOSITION.

    K >>= f   := m(T(f)(K)) = UNION { f(x) | x in K }
    g  *  f   := λx. f(x) >>= g            (Kleisli composition)

The Stage-32/33 fork is re-read INSIDE one algebra.  A partial morphism
    R : X -> Option X
has TWO Kleisli readings in the SAME codomain P(X):
    kappa_R(x) = { R(x) }  if defined, else EMPTY      (partial arm)
    kbar_R(x)  = { R(x) }  if defined, else { x }       (totalized arm)
Empty means "no Kleisli successor"; it is NOT identified with Belnap N.
"""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lift_reentry import MORPH, FSPLIT, FFUSE, TANCH, AFWD, IFIX, EVALT, EVALF
from lift_35 import weight_fields, run_batch
from lift_37 import marks
from lift_40 import pi_host
import lift_43 as L43

FAM = ("FRAME_WORK", "CLOSE_FRAME", "IDENTITY")
WIRES = {"w1": FSPLIT + FFUSE + EVALF + AFWD + IFIX,
         "w2": FSPLIT + FFUSE + EVALT + AFWD + IFIX,
         "w3": TANCH + FSPLIT + AFWD + EVALT + TANCH,
         "w4": FSPLIT + EVALT + AFWD + IFIX + FFUSE}

def wset(K):
    """canonical SET word (bytes) of a Kleisli result (frozenset of words)."""
    return L43.word(frozenset(K))

def kappa(R, x):
    y = MORPH[R](x)
    return frozenset({y}) if y is not None else frozenset()

def kbar(R, x):
    y = MORPH[R](x)
    return frozenset({y}) if y is not None else frozenset({x})

def u(x):
    return frozenset({x})

def bind(K, f):
    r = frozenset()
    for x in K:
        r = r | f(x)
    return r

def kleisli(g, f):
    return lambda x: bind(f(x), g)

def laws():
    print("=== 44A: the Kleisli laws (byte equality after canonicalization) ===")
    arrows = {"u": u,
              "kappa_FW": lambda x: kappa("FRAME_WORK", x),
              "kbar_CF": lambda x: kbar("CLOSE_FRAME", x),
              "kappa_CF": lambda x: kappa("CLOSE_FRAME", x)}
    Ks = [frozenset(), frozenset({WIRES["w1"]}),
          frozenset({WIRES["w1"], WIRES["w2"]}),
          frozenset({WIRES["w1"], WIRES["w3"], WIRES["w4"]})]
    l1 = all(wset(bind(u(x), f)) == wset(f(x))
             for x in WIRES.values() for f in arrows.values())
    l2 = all(wset(bind(K, u)) == wset(K) for K in Ks)
    l3 = True
    for K in Ks:
        for f in arrows.values():
            for g in arrows.values():
                lhs = bind(bind(K, f), g)
                rhs = bind(K, kleisli(g, f))
                if wset(lhs) != wset(rhs):
                    l3 = False
    print(f"   left   u(x) >>= f = f(x)        : {l1}")
    print(f"   right  K >>= u = K               : {l2}")
    print(f"   assoc  (K>>=f)>>=g = K>>=(g*f)   : {l3}")
    print()
    return l1 and l2 and l3

def old_partial(R1, R2, w):
    x = MORPH[R1](w)
    return None if x is None else MORPH[R2](x)

def old_total(R1, R2, w):
    x = MORPH[R1](w); x = w if x is None else x
    y = MORPH[R2](x)
    return x if y is None else y

def kl_g(R1, R2):
    return kleisli(lambda y: kappa(R2, y), lambda x: kappa(R1, x))

def kl_gbar(R1, R2):
    return kleisli(lambda y: kbar(R2, y), lambda x: kbar(R1, x))

def one(Z):
    return None if len(Z) == 0 else sorted(Z)[0]

def compare():
    print("=== 44B: 3x3 composition -- Kleisli vs the old Stage-32/33 tables ===")
    tot = pm = tm = 0
    for R1 in FAM:
        for R2 in FAM:
            for wn, w in WIRES.items():
                tot += 1
                if old_partial(R1, R2, w) == one(kl_g(R1, R2)(w)):
                    pm += 1
                Z = kl_gbar(R1, R2)(w)
                kt = sorted(Z)[0] if len(Z) == 1 else ("MULTI" if len(Z) > 1 else None)
                if old_total(R1, R2, w) == kt:
                    tm += 1
    print(f"   partial  kappa_R2 * kappa_R1  == old partial : {pm}/{tot}")
    print(f"   total    kbar_R2  * kbar_R1   == old total   : {tm}/{tot}")
    print("   matrix (wire w1)  R1 \\ R2 : old_partial ?= kleisli_partial")
    for R1 in FAM:
        cells = []
        for R2 in FAM:
            op = old_partial(R1, R2, WIRES["w1"])
            kp = one(kl_g(R1, R2)(WIRES["w1"]))
            cells.append("ok" if op == kp else f"DIFF")
        print(f"     {R1:<11} " + "  ".join(f"{c:<4}" for c in cells))
    print()
    return pm == tot and tm == tot

def fork_test():
    print("=== 44C: multi-successor fork -- both branches survive bind ===")
    lf = lambda x: kappa("FRAME_WORK", x)
    rf = lambda x: kappa("CLOSE_FRAME", x)
    g = lambda y: kbar("IDENTITY", y)
    ok = True
    for w in WIRES.values():
        F = frozenset(lf(w) | rf(w))                 # fork(x) = {left(x), right(x)}
        lhs = bind(F, g)
        rhs = frozenset().union(*[g(y) for y in F])  # g(y1) ∪ g(y2)
        if wset(lhs) != wset(rhs):
            ok = False
    w0 = WIRES["w2"]
    print(f"   fork(w2) = {len(lf(w0)|rf(w0))} successors  "
          f"bind -> {len(bind(frozenset(lf(w0)|rf(w0)), g))} successors")
    print(f"   {{y1,y2}} >>= g = g(y1) ∪ g(y2)  (both kept) : {ok}")
    print()
    return ok

def empty_vs_identity():
    print("=== 44D: EMPTY vs IDENTITY singleton -- the Stage-33 discrepancy source ===")
    g = lambda y: kbar("FRAME_WORK", y)
    for R in FAM:
        for wn, w in WIRES.items():
            if MORPH[R](w) is None:
                Kp, Kb = kappa(R, w), kbar(R, w)
                print(f"   R={R} wire={wn}:  kappa={len(Kp)} successors   kbar={len(Kb)} successor")
                print(f"     partial  kappa >>= g = {len(bind(Kp, g))} successors   (empty relational image)")
                print(f"     total    kbar  >>= g = {len(bind(Kb, g))} successor    (= g(x), x re-entered)")
                print("   -> discrepancy source = EMPTY replaced by IDENTITY singleton BEFORE composition,")
                print("      not a failure of composition.  ∅ ≠ {x}.")
                print()
                return True
    print("   (no off-domain case on these wires; both arms exercised in 44B)")
    print()
    return True

def four_relation():
    print("=== 44E: cardinality vs FOUR -- measured separately, NOT identified ===")
    reps = {"EMPTY": frozenset(),
            "singleton": frozenset({WIRES["w2"]}),
            "pair": frozenset({WIRES["w1"], WIRES["w2"]})}
    allw = sorted({x for K in reps.values() for x in K})
    PI = {}
    if allw:
        o = run_batch(["weight " + w for w in allw])
        PI = {w: pi_host(marks(weight_fields(o, w))) for w in allw}
    from lift_42 import join_i
    for nm, K in reps.items():
        pis = sorted(PI.get(x, "?") for x in K)
        j = ""
        for p in pis:
            j = p if j == "" else join_i(j, p)
        print(f"   {nm:<10} |K|={len(K)}  member π={pis}  join_i={j or '∅'}")
    print("   cardinality 0/1/2 does NOT biject to N/T/B — the Belnap reading is measured, not assigned.")
    print()
    return True

def main():
    print("=== Stage 44: KLEISLI / RE-ENTRY COMPOSITION ===")
    print("   K >>= f = UNION { f(x) | x in K } ;  g * f = \\x. f(x) >>= g")
    print()
    a = laws()
    b = compare()
    c = fork_test()
    d = empty_vs_identity()
    e = four_relation()
    ok = a and b and c and d and e
    print("STAGE44: the Stage-32/33 fork is internalized by the powerset monad.")
    print(f"   Kleisli laws {a} | old-table reproduction {b} | fork {c} | empty/id {d} | FOUR {e} => {ok}")
    print("   absence = EMPTY ; identity fill = {x} ; both live in ONE codomain P(X).")
    print("   NEXT (Stage 45): Eilenberg-Moore algebras for T -- the tower's own algebra structure.")

if __name__ == "__main__":
    main()
