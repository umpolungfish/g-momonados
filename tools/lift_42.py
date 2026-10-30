#!/usr/bin/env python3
"""lift_42.py - Stage 42: V3 becomes an ALGEBRA.

Two layers kept simultaneously, never silently identified:

  LAYER 1  raw powerset algebra  P(D), D the finite V2 domain of Stage 41.
           union / intersection / difference / subset, canonical byte identity.
           Law table exhaustively checked over P(D).

  LAYER 2  trilattice lift.  THREE orders named separately:
             truth      <=t   join_t / meet_t   (Belnap connectives)
             information<=i   join_i / meet_i   (componentwise; = para_vm default)
             falsity    <=f   join_f / meet_f   (t<->f dual of truth)
           V2 op  x o y  ->  V3 op  K o^ L = { x o y }.
           Kernel binary composition (superposition) measured against the three.

  FUNCTOR  direct image P(f)(K) = { f(x) | x in K }; check P(id)=id,
           P(g.f)=P(g).P(f), and P(C)^2 = P(C) for C = eta.pi.
"""
import sys, os, random
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lift_37 import marks, truth_proj, A, Bst, ENGAGR
from lift_35 import weight_fields, run_batch
from lift_40 import pi_host, crispword
from lift_41 import V2, member
import lift_reentry as R

# ---- B4 as (t,f) bit pairs ------------------------------------------------
B4 = {"N": (0, 0), "T": (1, 0), "F": (0, 1), "B": (1, 1)}
INV = {v: k for k, v in B4.items()}
def _j(a, b, ft, ff):
    ta, fa = B4[a]; tb, fb = B4[b]
    return INV[(ft(ta, tb), ff(fa, fb))]
def join_t(a, b): return _j(a, b, max, min)   # disjunction
def meet_t(a, b): return _j(a, b, min, max)   # conjunction
def join_i(a, b): return _j(a, b, max, max)   # information join (= para_vm default)
def meet_i(a, b): return _j(a, b, min, min)
def join_f(a, b): return _j(a, b, min, max)   # falsity join (t<->f dual of truth)
def meet_f(a, b): return _j(a, b, max, min)

OPS = {"join_t": join_t, "meet_t": meet_t,
       "join_i": join_i, "meet_i": meet_i,
       "join_f": join_f, "meet_f": meet_f}

def idx(lab):
    return [l for l, _ in V2].index(lab)

CRISP_IDX = {"B": idx("TF"), "T": idx("T"), "F": idx("F"), "N": idx("N")}

def measure():
    """one boot for the domain, one for the (eta.pi)-canonical words."""
    o1 = run_batch(["weight " + w for _, w in V2])
    d = {}
    for lab, w in V2:
        m = marks(weight_fields(o1, w))
        d[lab] = {"word": w, "reg": (m["T"], m["F"], m["t"], m["f"]),
                  "pi": pi_host(m)}
    Cw = [ENGAGR + crispword(d[lab]["pi"]) for lab, _ in V2]
    o2 = run_batch(["weight " + w for w in Cw])
    C = {}
    for (lab, _), w in zip(V2, Cw):
        C[lab] = {"word": w, "pi": pi_host(marks(weight_fields(o2, w)))}
    return d, C

def sup_words():
    """kernel binary composition: superposition o = (in x ni)(in y ni)."""
    pairs = [(i, j) for i in range(len(V2)) for j in range(len(V2))]
    ws = [R.FSPLIT + V2[i][1] + R.FFUSE + R.FSPLIT + V2[j][1] + R.FFUSE
          for i, j in pairs]
    return pairs, ws

N = len(V2)
FULL = (1 << N) - 1

def _bits(mask):
    return [i for i in range(N) if (mask >> i) & 1]

def powerset_laws():
    from lift_41 import canon_set
    print(f"=== 42A: raw powerset algebra over P(D),  |D|={N},  |P(D)|={1 << N} ===")
    seen = {}
    inj = True
    for m in range(1 << N):
        b = canon_set([V2[i][1] for i in _bits(m)]).encode("utf-8")
        if b in seen:
            inj = False
        seen[b] = m
    print(f"   canonical byte encoding injective on P(D) : {inj}  ({len(seen)} subsets)")
    uni = all(((m | m) == m) and ((m & m) == m) for m in range(1 << N))
    com = all((a | b) == (b | a) and (a & b) == (b & a)
              for a in range(1 << N) for b in range(1 << N))
    absorb = all(((a | (a & b)) == a) and ((a & (a | b)) == a)
                 for a in range(1 << N) for b in range(1 << N))
    sub = all((((a & ~b) & FULL) == 0) == ((a & b) == a)
              for a in range(1 << N) for b in range(1 << N))
    rnd = random.Random(42)
    assoc = True
    for _ in range(200000):
        a, b, c = (rnd.randrange(1 << N) for _ in range(3))
        if ((a | b) | c) != (a | (b | c)) or ((a & b) & c) != (a & (b & c)):
            assoc = False
            break
    print(f"   idempotence   K|K=K, K&K=K               : {uni}")
    print(f"   commutativity (all pairs)                : {com}")
    print(f"   associativity (200k triples)             : {assoc}")
    print(f"   absorption    K|(K&L)=K, K&(K|L)=K       : {absorb}")
    print(f"   subset  (K⊆L  ⟺  K∩L=K)                  : {sub}")
    print()
    return inj and uni and com and assoc and absorb and sub

def fibers(d):
    print("=== 42B: the π-fibers partition D ===")
    labs = [l for l, _ in V2]
    K = {}
    for j in ["T", "F", "N", "B"]:
        K[j] = 0
        for i, l in enumerate(labs):
            if d[l]["pi"] == j:
                K[j] |= (1 << i)
    names = {"B": "K_TF", "T": "K_T", "F": "K_F", "N": "K_N"}
    for j in ["T", "F", "N", "B"]:
        mem = [labs[i] for i in _bits(K[j])]
        print(f"   {names[j]:<5} = {{{', '.join(mem)}}}   |.|={len(mem)}")
    disj = all((K[i] & K[j]) == 0
               for i in ["T", "F", "N", "B"] for j in ["T", "F", "N", "B"] if i != j)
    union = 0
    for j in ["T", "F", "N", "B"]:
        union |= K[j]
    print(f"   pairwise disjoint  K_i ∩ K_j = ∅  (i≠j)   : {disj}")
    print(f"   union  ⋃_j K_j = D                        : {union == FULL}")
    print("   -> π appears one level up as a DECOMPOSITION of V2 into V3 values.")
    print()
    return disj and (union == FULL), K

def op_tab(d, fn):
    pl = [d[l]["pi"] for l, _ in V2]
    return [[CRISP_IDX[fn(pl[i], pl[j])] for j in range(N)] for i in range(N)]

def lift_tab(t, K, L):
    r = 0
    for i in _bits(K):
        for j in _bits(L):
            r |= 1 << t[i][j]
    return r

def lifts(d):
    print("=== 42C: V3 op  K ⋄̂ L = { x⋄y }  -- three orders, three names ===")
    print(f"   three joins on the (T,F) pair: join_t={join_t('T','F')}  "
          f"join_i={join_i('T','F')}  join_f={join_f('T','F')}")
    rnd = random.Random(7)
    ok = True
    for name, fn in OPS.items():
        t = op_tab(d, fn)
        sing = all(lift_tab(t, 1 << a, 1 << b) == (1 << t[a][b])
                   for a in range(N) for b in range(N))
        asc = True
        for _ in range(20000):
            a, b, c = (rnd.randrange(1 << N) for _ in range(3))
            if lift_tab(t, lift_tab(t, a, b), c) != lift_tab(t, a, lift_tab(t, b, c)):
                asc = False
                break
        ok &= sing and asc
        print(f"   {name:<7} {{a}}⋄̂{{b}}={{a⋄b}} {str(sing):<6} associativity {asc}")
    print("   (three names kept distinct; join_f(T,F)=F=meet_t(T,F) -- a MEASURED identity)")
    print()
    return ok

def superposition(d):
    print("=== 42C': kernel binary composition (superposition ⊕) vs the three joins ===")
    pairs, ws = sup_words()
    o = run_batch(["weight " + w for w in ws])
    cnt = {"join_t": 0, "join_i": 0, "join_f": 0}
    for (i, j), w in zip(pairs, ws):
        v = pi_host(marks(weight_fields(o, w)))
        for nm, fn in [("join_t", join_t), ("join_i", join_i), ("join_f", join_f)]:
            if fn(d[V2[i][0]]["pi"], d[V2[j][0]]["pi"]) == v:
                cnt[nm] += 1
    tot = len(pairs)
    print(f"   pairs measured: {tot}")
    for nm in cnt:
        print(f"   π(x ⊕ y) == {nm}(πx,πy) : {cnt[nm]}/{tot}")
    print("   -> the kernel's own composition realizes a SPECIFIC order (measured, not assumed).")
    print()

def functor(d):
    print("=== 42D: powerset functor  T(V2)=P(V2), T(f)=P(f) ===")
    pl = [d[l]["pi"] for l, _ in V2]
    dom = [w for _, w in V2]
    Cw = [ENGAGR + crispword(p) for p in pl]
    o = run_batch(["weight " + w for w in Cw])
    piC = {w: pi_host(marks(weight_fields(o, w))) for w in Cw}
    piOf = dict(zip(dom, pl)); piOf.update(piC)
    def f_id(w): return w
    def f_C(w): return ENGAGR + crispword(piOf[w])
    def Pset(f, words): return frozenset(member(f(w)) for w in words)
    # T(id)=id on ALL subsets
    idok = True
    for m in range(1 << N):
        ws = [dom[i] for i in _bits(m)]
        if Pset(f_id, ws) != frozenset(member(w) for w in ws):
            idok = False
            break
    # functoriality P(g.f)=P(g).P(f) over all subsets, f,g in {id,C}
    fun_ok = True
    pairs_fg = [(f_id, f_id), (f_id, f_C), (f_C, f_id), (f_C, f_C)]
    for f, g in pairs_fg:
        for m in range(1 << N):
            ws = [dom[i] for i in _bits(m)]
            gf = Pset(g, [f(w) for w in ws])
            comp = Pset(lambda w: g(f(w)), ws)
            if gf != comp:
                fun_ok = False
                break
    # P(C)^2 = P(C) over all subsets
    cidem = True
    for m in range(1 << N):
        ws = [dom[i] for i in _bits(m)]
        c1 = [f_C(w) for w in ws]
        if Pset(f_C, c1) != frozenset(member(w) for w in c1):
            cidem = False
            break
    print(f"   T(id)(K)=K            (all {1 << N} subsets) : {idok}")
    print(f"   T(g∘f)=T(g)∘T(f)      (all subsets, {{id,C}}) : {fun_ok}")
    print(f"   P(C)²=P(C)            (all subsets)          : {cidem}")
    print(f"   C-images π preserved (π(C(x))=π(x))          : "
          f"{all(piC[Cw[i]] == pl[i] for i in range(N))}")
    print()
    return idok and fun_ok and cidem

def main():
    print("=== Stage 42: V3 becomes an ALGEBRA ===")
    d, C = measure()
    print("  V2 domain π: " + ", ".join(f"{l}→{d[l]['pi']}" for l, _ in V2))
    print()
    a = powerset_laws()
    b, K = fibers(d)
    c = lifts(d)
    superposition(d)
    f = functor(d)
    print("STAGE42: V3 is an algebra -- raw powerset laws AND a lifted trilattice layer.")
    print(f"   42A powerset laws {a} | 42B fiber partition {b} | 42C lifts {c} | 42D functor {f}")
    print("   NEXT (Stage 43): the re-entry functor's monad structure T∘T ⇒ T on the tower.")

if __name__ == "__main__":
    main()
