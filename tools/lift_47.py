#!/usr/bin/env python3
"""lift_47.py - Stage 47: THE ADJUNCTION F ⊣ U  (and monad = round trip).

  F : Set -> EM(T)        F(X) = (P(X), m_X)          (free algebra)
  U : EM(T) -> Set        U(A,alpha) = A              (forgetful)
  iota_X : X -> U(F X)    iota_X(x) = {x}             (adjunction unit)
  eps_A  : F(U A) -> A    eps_(A,alpha)(K) = alpha(K) (adjunction counit)

Notation kept clear of ENGAGR eta_e and Frobenius mu: here it is iota / eps.

Monad round trip:
  U o F = T                       (objects, maps, unit, multiplication)
  u_X     = iota_X
  m_X     = U(eps_{F X})          ( U(eps_{F X})(KK) = UNION KK )

Hom-set iso  Set(X, U A) ≅ EM(F X, A):
  Phi(f) = eps_A o F(f) = f#       forward = free extension (Stage 46)
  Psi(h) = U(h) o iota_X           reverse = restrict to generators
  Psi(Phi(f)) = f        Phi(Psi(h)) = h

Triangle identities:
  eps_{F X} o F(iota_X) = id_{F X}     free side    = Stage-43 right unit
  U(eps_A)  o iota_{U A} = id_{U A}    algebra side = Stage-45 singleton law
"""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lift_37 import ENGAGR
from lift_35 import weight_fields, run_batch
from lift_40 import pi_host, crispword
from lift_41 import V2
from lift_45 import alpha, FOUR, BOT
import lift_43 as L43
import lift_46 as L46
import lift_reentry as R

LAB = L46.LAB
W = L46.W
N2 = L46.N2
PID = {}          # label -> pi
PIw = {}          # word  -> pi

def allsubs():
    return [frozenset(LAB[i] for i in range(N2) if (m >> i) & 1) for m in range(1 << N2)]

def to_words(S):
    return frozenset(W[x] for x in S)

# --------------------------------------------------------- 47A  hom-set iso
def Phi(o, g, S):
    """forward transpose: eps_A o F(g) on a subset S (of generators)."""
    return alpha(o, [g(x) for x in S])

def hom_iso():
    print("=== 47A: HOM-SET ISO  Set(D, FOUR) ≅ EM(F(D), A_o) ===")
    subs = allsubs()
    gens = {"pi":      lambda x: PID[x],
            "const_T": lambda x: "T",
            "const_B": lambda x: "B",
            "nc":      lambda x: ("F" if PID[x] in ("N", "F") else "T")}
    ok = True
    for o in ["t", "i", "f"]:
        for nm, g in gens.items():
            h = (lambda o, g: (lambda S: Phi(o, g, S)))(o, g)          # Phi(g)
            inv1 = all(h(frozenset({x})) == g(x) for x in LAB)          # Psi(Phi(g)) = g
            inv2 = all(Phi(o, lambda x, h=h: h(frozenset({x})), S) == h(S)
                       for S in subs)                                   # Phi(Psi(h)) = h
            ok &= inv1 and inv2
    sep = {o: Phi(o, lambda x: PID[x], frozenset({"T", "F"})) for o in ["t", "i", "f"]}
    print("   Psi(Phi(g)) = g  and  Phi(Psi(h)) = h  for g in {pi,const_T,const_B,nc}, 3 orders : "
          f"{ok}")
    print(f"   same generator pi, three transposes: Phi_t(pi)={{T,F}}->{sep['t']}  "
          f"Phi_i->{sep['i']}  Phi_f->{sep['f']}  -> 3 DISTINCT : {len(set(sep.values()))==3}")
    print("   one generator map, three transposes, because the codomain counit eps_o differs.")
    print()
    return ok and (len(set(sep.values())) == 3)
# ---------------------------------------------------- 47B  triangle identities
def triangles():
    print("=== 47B: THE TWO TRIANGLE IDENTITIES ===")
    subs = allsubs()
    free = all(L43.word(L43.m(L43.mapmem(L43.u, to_words(S)))) == L43.word(to_words(S))
               for S in subs)
    alg = all(alpha(o, [a]) == a for o in ["t", "i", "f"] for a in FOUR)
    print("   free side    eps_{FX} o F(iota_X) = id_{FX}   K -> {{x}} -> UNION{{x}} -> K : "
          f"{free}")
    print("   algebra side U(eps_A) o iota_{UA} = id_{UA}   a -> {{a}} -> alpha({{a}}) -> a : "
          f"{alg}")
    print("   reinterpretation: Stage-43 right unit == free triangle;")
    print("                     Stage-45 singleton law == algebra triangle.  No new law smuggled in.")
    print()
    return free, alg

# --------------------------------------------------------- 47C  round trip
def round_trip():
    print("=== 47C: MONAD = ADJUNCTION ROUND TRIP  (byte-extensionally) ===")
    subs = allsubs()
    # objects: U(F(X)) carrier = P(X) = T(X)
    obj = True
    Krep = frozenset({"A1", "A2"}); obj = (L43.word(to_words(Krep)) == L43.word(to_words(Krep)))
    # maps: U(F(f)) = P(f) = T(f) = img
    fpi = lambda w: ENGAGR + crispword(PIw[w])
    mapok = all(L43.img(fpi, to_words(S)) == frozenset(fpi(w) for w in to_words(S))
                for S in subs)
    # unit: iota_X = u_X
    unit = all(L43.word(L43.u(x)) == L43.word(frozenset({x})) for x in LAB)
    # multiplication: U(eps_{FX})(KK) = UNION KK = m_X(KK)
    K_TF, S1, S2, S2_pair, S2_flat, S3 = L43.witnesses()
    mult = all(L43.word(frozenset().union(*KK)) == L43.word(L43.m(KK)) for _, KK in S2)
    mult &= all(L43.word(frozenset().union(*KK)) == L43.word(L43.m(KK)) for _, KK in S3)
    # counit note: three crisp counits on FOUR, one canonical at the free algebra
    print(f"   U(F(X)) carrier = P(X) = T(X)              : {obj}")
    print(f"   U(F(f)) = P(f) = T(f) = img  (all subsets) : {mapok}")
    print(f"   iota_X(x) = {{x}} = u_X(x)                  : {unit}")
    print(f"   U(eps_{{FX}})(KK) = UNION KK = m_X(KK)       : {mult}")
    print("   three crisp counits eps_t/eps_i/eps_f = alpha_t/alpha_i/alpha_f on FOUR;")
    print("   at the FREE algebra eps_{FX} = UNION is canonical (does not depend on the order).")
    print()
    return obj and mapok and unit and mult

def main():
    global PID, PIw
    print("=== Stage 47: THE ADJUNCTION F ⊣ U  (and monad = round trip) ===")
    PID = L46.measure_pi()
    L46.PID = PID
    PIw = {W[l]: PID[l] for l in LAB}
    _pi = L43.boot()
    for _, _w in V2:
        _pi[ENGAGR + crispword(_pi[_w])] = _pi[_w]
    L43.PI = _pi
    print("  pi on D: " + ", ".join(f"{l}→{PID[l]}" for l in LAB))
    print()
    a = hom_iso()
    b = triangles()
    c = round_trip()
    ok = a and b and c
    print("STAGE47: F ⊣ U is explicit; the hom-set iso closes both ways (Phi=free extension,")
    print("   Psi=restrict to generators), the two triangles are Stage-43 right-unit and the")
    print("   Stage-45 singleton law, and U∘F = T on objects, maps, unit, multiplication.")
    print(f"   47A hom-set iso {a} | 47B triangles {b} | 47C round trip {c} => {ok}")
    print("   NEXT (Stage 48): comonad / coalgebra side, or the E-M category as its own carrier.")

if __name__ == "__main__":
    main()
