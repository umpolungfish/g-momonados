#!/usr/bin/env python3
"""lift_43.py - Stage 43: the re-entry functor closes as a MONAD.

    T(X)   = P(X)                       (powerset functor; Stage 42)
    u_X(x) = { x }                      powerset UNIT       (NOT eta_e : V1->V2)
    m_X(K) = UNION K                    powerset MULTIPLY   (NOT mu : Frobenius)

Nested-set discipline (Stage-41 style, byte-extensional):
    member-of-SET2 = wrap(SET word)     wrap(w) = in LEN8(w) ni in w ni
    SET2           = canon(sorted member words)
Value identity = the canonical word bytes.  Nesting is NOT flattened in encoding:
    {{A1},{A2}}  !=  {{A1,A2}}    (different at P^2)
yet both give  {A1,A2}  after m -- that non-injectivity IS the multiplication.
"""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lift_37 import marks, A, Bst, ENGAGR
from lift_35 import weight_fields, run_batch
from lift_40 import pi_host, crispword
from lift_41 import V2
import lift_reentry as R

def _len8(b):
    return "".join(R.EVALT if x == "1" else R.EVALF
                   for x in format(len(b.encode("utf-8")) & 0xFF, "08b"))

def wrap(w):
    return R.FSPLIT + _len8(w) + R.FFUSE + R.FSPLIT + w + R.FFUSE

def word(spec):
    """canonical word-bytes of a set/atom spec (atom = a V2 word string)."""
    if isinstance(spec, str):
        return spec
    return R.FSPLIT + "".join(sorted(wrap(word(e)) for e in spec)) + R.FFUSE

def m(spec):
    """multiplication m_X(K) = UNION K  (removes one powerset nesting)."""
    r = frozenset()
    for c in spec:
        r = r | c
    return r

def _img(f, e):
    return f(e) if isinstance(e, str) else img(f, e)

def img(f, spec):
    """direct image T(f): replace every atom by f(atom), preserve nesting."""
    if isinstance(spec, str):
        return f(spec)
    return frozenset(_img(f, e) for e in spec)

def mapmem(g, spec):
    """apply g to each IMMEDIATE member (the one-level-up functorial action)."""
    return frozenset(g(e) for e in spec)

def u(x):
    return frozenset({x})

def boot():
    o = run_batch(["weight " + w for _, w in V2])
    pi = {w: pi_host(marks(weight_fields(o, w))) for _, w in V2}
    return pi

WD = {l: w for l, w in V2}
def C(w):
    return ENGAGR + crispword(PI[w])

PI = {}

def witnesses():
    wd = WD
    K_T = frozenset({wd["T"]}); K_F = frozenset({wd["F"]})
    K_N = frozenset({wd["tf"], wd["N"]})
    K_TF = frozenset({wd[l] for l in ["TF", "A1", "A2", "A3", "A4", "A5"]})
    S1 = [("∅", frozenset()), ("{A1}", frozenset({wd["A1"]})),
          ("K_T", K_T), ("K_TF", K_TF), ("K_N", K_N)]
    S2_pair = frozenset({frozenset({wd["A1"]}), frozenset({wd["A2"]})})
    S2_flat = frozenset({frozenset({wd["A1"], wd["A2"]})})
    S2 = [("{∅}", frozenset({frozenset()})),
          ("{{A1}}", frozenset({frozenset({wd["A1"]})})),
          ("{{A1},{A2}}", S2_pair),
          ("{{A1,A2}}", S2_flat),
          ("{K_T,K_F,K_N,K_TF}", frozenset({K_T, K_F, K_N, K_TF})),
          ("{K_TF,{A1},{TF}}", frozenset({K_TF, frozenset({wd["A1"]}),
                                          frozenset({wd["TF"]})})),
          ("{K_TF,P(C)(K_TF)}", frozenset({K_TF, img(C, K_TF)}))]
    S3 = [("X1", frozenset({S2_pair, frozenset({frozenset({wd["A3"]})})})),
          ("X2", frozenset({frozenset({K_TF, K_N}), frozenset({K_T, K_F})})),
          ("X3", frozenset({frozenset({frozenset({wd["A1"]})}), S2_flat}))]
    return K_TF, S1, S2, S2_pair, S2_flat, S3

def checks():
    print("=== 43A: nested-SET representation (byte-extensional) ===")
    K_TF, S1, S2, S2_pair, S2_flat, S3 = witnesses()
    S2_perm = frozenset({frozenset({WD["A2"]}), frozenset({WD["A1"]})})
    S2_dup = frozenset({frozenset({WD["A1"]}), frozenset({WD["A1"]}),
                        frozenset({WD["A2"]})})
    print(f"   order invariance  {{K1,K2}}=={{K2,K1}}   : {word(S2_pair) == word(S2_perm)}")
    print(f"   duplicate idempot {{K1,K1,K2}}=={{K1,K2}}: {word(S2_dup) == word(S2_pair)}")
    print(f"   NOT flattened     {{A1}},{{A2}} != {{A1,A2}} : {word(S2_pair) != word(S2_flat)}")
    print(f"   m flattens both   m({{A1}},{{A2}})==m({{A1,A2}}) : "
          f"{word(m(S2_pair)) == word(m(S2_flat))}")
    print()
    print("=== 43B: the three monad laws ===")
    lu = all(word(m(u(K))) == word(K) for _, K in S1)
    ru = all(word(m(mapmem(lambda x: frozenset({x}), K))) == word(K) for _, K in S1)
    assoc = all(word(m(mapmem(m, X))) == word(m(m(X))) for _, X in S3)
    print(f"   LEFT  unit   m ∘ u_TX = id_TX   : {lu}")
    print(f"   RIGHT unit   m ∘ T(u) = id       : {ru}")
    print(f"   ASSOC        m ∘ T(m) = m ∘ m_TD : {assoc}")
    print()
    print("=== 43C: monad naturality (f = C = eta_e∘pi) ===")
    nat = all(word(img(C, m(KK))) == word(m(img(C, KK))) for _, KK in S2)
    unat = all(word(img(C, u(x))) == word(u(C(x))) for x in WD.values())
    print(f"   mult naturality  T(f)∘m = m∘T²(f) : {nat}")
    print(f"   unit naturality  T(f)∘u = u∘f     : {unat}")
    print()
    return lu, ru, assoc, nat, unat

def ground(K_TF):
    print("=== 43D: kernel grounding -- nested SET2 words are valid & close ===")
    w1 = word(frozenset({frozenset({WD["A1"]}), frozenset({WD["A2"]})}))
    w2 = word(frozenset({frozenset({WD["A1"], WD["A2"]})}))
    w3 = word(m(frozenset({frozenset({WD["A1"]}), frozenset({WD["A2"]})})))
    o = run_batch(["weight " + x for x in [w1, w2, w3]])
    for nm, w in [("SET2 {{A1},{A2}}", w1), ("SET2 {{A1,A2}}", w2), ("m(SET2)", w3)]:
        f = weight_fields(o, w)
        print(f"   {nm:<18} final={f.get('final')}  surviving={f.get('surviving')}")
    print()

def main():
    global PI
    PI = boot()
    for _, w in V2:                      # make C total: pi(C(w)) = pi(w)
        PI[ENGAGR + crispword(PI[w])] = PI[w]
    print("=== Stage 43: the re-entry functor closes as a MONAD ===")
    print("   T(X)=P(X),  u_X(x)={x},  m_X(K)=UNION K")
    print()
    K_TF, S1, S2, S2_pair, S2_flat, S3 = witnesses()
    res = checks()
    ground(K_TF)
    ok = all(res)
    print("STAGE43: powerset monad present operationally.")
    print(f"   left unit {res[0]} | right unit {res[1]} | assoc {res[2]} | "
          f"mult-nat {res[3]} | unit-nat {res[4]}  => {ok}")
    print("   Non-injectivity {{A1},{A2}} != {{A1,A2}} yet equal after m: a held distinction.")
    print("   NEXT (Stage 44): Kleisli/re-entry composition -- the monad's bind on the tower.")

if __name__ == "__main__":
    main()
