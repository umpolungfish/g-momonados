#!/usr/bin/env python3
"""lift_49.py - Stage 49: E-M ALGEBRAS BECOME CONTENT.

A finite resident SUBCARRIER of EM(T), encoded with Stage-41 discipline:

    ALG(A,alpha) := ( carrier encoding A,  algebra/counit encoding alpha )
    ALG_MEMBER   := length-framed canonical ALG word
    ALG_SET      := canonical set of ALG_MEMBERs

The second coordinate is essential (Stage 48's witness):

    U(A_t)=U(A_i)=U(A_f)=FOUR      G(A_t)=G(A_i)=G(A_f)=F(FOUR)
    AND   eps_t != eps_i != eps_f

so the three are same-on-carrier, same-under-G, DIFFERENT as E-M objects.

    E0 = { ALG(FOUR,alpha_t), ALG(FOUR,alpha_i), ALG(FOUR,alpha_f), ALG(P(D),m_D) }

Three identity relations measured separately:  ≈U (same carrier), ≈G (same
G-image), =EM (same algebra/counit).  Decisive witness:
    K_EM={A_t,A_i,A_f}   U collapses it, G collapses it, eps SEPARATES it.

This is no longer "states become members" (Stage 41) but
"ways of evaluating re-entered states become members".
"""
import sys, os, itertools
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lift_37 import ENGAGR
from lift_35 import weight_fields, run_batch
from lift_41 import V2, member, canon_set
from lift_45 import alpha, FOUR, BOT
import lift_43 as L43
import lift_46 as L46
import lift_44 as L44
import lift_reentry as R

LAB = L46.LAB
W = L46.W
ORD3 = ["t", "i", "f"]
word = L43.word
UNION_TAG = ENGAGR          # marker for the union counit

def pairw(a, b):
    return R.FSPLIT + a + R.FFUSE + R.FSPLIT + b + R.FFUSE

def alg_word(carrier, counit):
    return member(pairw(carrier, counit))

def alg_set(algs):
    return canon_set([alg_word(*a) for a in algs])

def four_subsets():
    return [frozenset(FOUR[i] for i in range(4) if (m >> i) & 1) for m in range(16)]

def alpha_graph(o):
    """encode alpha_o by its full graph  { K |-> alpha_o(K) : K subset FOUR }."""
    items = [member(pairw(word(K), alpha(o, sorted(K)))) for K in four_subsets()]
    return R.FSPLIT + "".join(items) + R.FFUSE

def m_graph():
    """encode m_D by its graph on a canonical sample of families."""
    sample = [frozenset(), frozenset({W["T"]}), frozenset({W["T"], W["F"]}),
              frozenset(W[l] for l in LAB)]
    items = []
    for K in sample:
        fam = R.FSPLIT + "".join(member(word(frozenset({x}))) for x in sorted(K)) + R.FFUSE
        items.append(member(pairw(fam, word(K))))
    return R.FSPLIT + "".join(items) + R.FFUSE

FOUR_CARRIER = word(frozenset(FOUR))
PD_CARRIER = word(frozenset(W[l] for l in LAB))
A_t = (FOUR_CARRIER, alpha_graph("t"))
A_i = (FOUR_CARRIER, alpha_graph("i"))
A_f = (FOUR_CARRIER, alpha_graph("f"))
A_D = (PD_CARRIER, m_graph())
E0 = [A_t, A_i, A_f, A_D]

def G_enc(a):
    """G(A)=F(U A)=(P(U A), union); U(A)=carrier."""
    return member(pairw(word(frozenset({a[0]})), UNION_TAG))

# ------------------------------------------------------- 49A  algebras as content
def content():
    print("=== 49A: E-M ALGEBRAS BECOME CONTENT ===")
    print("   ALG(A,alpha)=(carrier enc, counit enc);  ALG_MEMBER=length-framed ALG word")
    KEM = [A_t, A_i, A_f]
    U_all = all(a[0] == b[0] for a, b in itertools.combinations(KEM, 2))
    G_all = all(G_enc(a) == G_enc(b) for a, b in itertools.combinations(KEM, 2))
    EM_all = all(alg_word(*a) == alg_word(*b) for a, b in itertools.combinations(KEM, 2))
    print(f"   ≈U  same underlying carrier : all three equal    : {U_all}")
    print(f"   ≈G  same G-image            : all three equal    : {G_all}")
    print(f"   =EM same algebra/counit     : all three DISTINCT : {not EM_all}")
    Uhat = {a[0] for a in KEM}
    Ghat = {G_enc(a) for a in KEM}
    Esep = sorted({alpha(o, ["T", "F"]) for o in ORD3})
    print(f"   Û(K_EM) = {{FOUR}}       -> |.| = {len(Uhat)}")
    print(f"   Ĝ(K_EM) = {{F(FOUR)}}    -> |.| = {len(Ghat)}")
    print(f"   E(K_EM,{{T,F}}) = {Esep}              -> |.| = {len(Esep)}")
    print("   U collapses the triple AND G collapses it AND eps separates it completely.")
    print(f"   |E0| = {len(E0)}  (three crisp FOUR algebras + ALG(P(D),m_D)); "
          f"ALG_SET bytes = {len(alg_set(E0))}")
    print("   now the previous-level algebra structures are CONTENT of one object --")
    print("   ways of evaluating re-entered states become members (higher-order move).")
    print()
    return U_all and G_all and (not EM_all) and len(Uhat) == 1 and len(Esep) == 3

# --------------------------------------------------------- 49B  morphisms
def is_hom(h, o, p):
    idx = {x: i for i, x in enumerate(FOUR)}
    for K in four_subsets():
        if h[idx[alpha(o, sorted(K))]] != alpha(p, sorted({h[idx[x]] for x in K})):
            return False
    return True

def morphisms():
    print("=== 49B: E-M MORPHISMS AS CONTENT -- a finite subcategory ===")
    maps = list(itertools.product(FOUR, repeat=4))
    idx = {x: i for i, x in enumerate(FOUR)}
    idmap = tuple(FOUR)
    def comp(g, h):
        return tuple(g[idx[h[i]]] for i in range(4))
    H = {(o, p): [h for h in maps if is_hom(h, o, p)] for o in ORD3 for p in ORD3}
    sizes = "  ".join(f"{o}->{p}:{len(H[(o, p)])}" for o in ORD3 for p in ORD3)
    ids = all(idmap in H[(o, o)] for o in ORD3)
    comp_ok = True
    for o in ORD3:
        for p in ORD3:
            for q in ORD3:
                for h in H[(o, p)]:
                    for g in H[(p, q)]:
                        if comp(g, h) not in H[(o, q)]:
                            comp_ok = False
    assoc = True
    for o in ORD3:
        for p in ORD3:
            for q in ORD3:
                for r in ORD3:
                    for f in H[(p, q)]:
                        for g in H[(q, r)]:
                            for h in H[(o, p)]:
                                if comp(f, comp(g, h)) != comp(comp(f, g), h):
                                    assoc = False
    idl = all(comp(idmap, h) == h and comp(h, idmap) == h
              for o in ORD3 for p in ORD3 for h in H[(o, p)])
    print(f"   hom counts  {sizes}")
    print(f"   source/target typing by construction; identity in every H[o->o] : {ids}")
    print(f"   identity laws  id∘h=h=h∘id                                     : {idl}")
    print(f"   composition closed (hom-law preserved)                         : {comp_ok}")
    print(f"   associativity                                                  : {assoc}")
    print("   => Ob(E0)=the E-M algebras, Mor(E0)=the hom-law maps: a finite subcategory.")
    print()
    return ids and idl and comp_ok and assoc

# ------------------------------------------------- 49C  Kleisli comparison image
def comparison_image():
    print("=== 49C: J's IMAGE -- free algebras as objects, extended arrows ===")
    ws = list(L44.WIRES.values())
    arrows = {"u": lambda x: frozenset({x}),
              "kappa_FW": lambda x: L44.kappa("FRAME_WORK", x),
              "kbar_FW": lambda x: L44.kbar("FRAME_WORK", x)}
    subsets = [frozenset(ws[i] for i in range(len(ws)) if (m >> i) & 1)
               for m in range(1 << len(ws))]
    Jk = L46.J
    # hom law for the free-algebra arrow J(k): J(k)(m_X(fam)) = m_Y(P(J(k))(fam))
    hom = True
    for nm, k in arrows.items():
        if L43.word(Jk(k, frozenset())) != L43.word(frozenset()):
            hom = False
        for A in subsets:
            for Bq in subsets:
                if L43.word(Jk(k, A | Bq)) != L43.word(Jk(k, A) | Jk(k, Bq)):
                    hom = False
    jid = all(L43.word(Jk(arrows["u"], K)) == L43.word(K) for K in subsets)
    comp = True
    ks = list(arrows.values())
    for f in ks:
        for g in ks:
            gsf = L44.kleisli(g, f)
            for K in subsets:
                if L43.word(Jk(gsf, K)) != L43.word(Jk(g, Jk(f, K))):
                    comp = False
    print("   on objects  J(X) = F(X) -- the free algebra (in E0 as ALG(P(D), m_D))")
    print(f"   J(k)=K>>=k is an EM-arrow F(X)->F(Y): hom law J(k)∘m_X = m_Y∘P(J(k)) : {hom}")
    print(f"   J(u)=id  {jid} ;  J(g⋆f)=J(g)∘J(f)  {comp}")
    print("   image = free algebras together with their extended arrows; J lands there.")
    print()
    return hom and jid and comp

def main():
    print("=== Stage 49: E-M ALGEBRAS BECOME CONTENT ===")
    a = content()
    b = morphisms()
    c = comparison_image()
    ok = a and b and c
    print("STAGE49: E0 = {A_t,A_i,A_f,ALG(P(D),m_D)} is a finite resident subcarrier of EM(T);")
    print("   U and G collapse the crisp triple while eps SEPARATES it -- a held higher object.")
    print(f"   49A content {a} | 49B subcategory {b} | 49C J-image {c} => {ok}")
    print("   NEXT (Stage 50): the comparison functor Kl(T)->EM(T) as a full subcategory inclusion,")
    print("   or the E-M objects' own internal re-entry.")

if __name__ == "__main__":
    main()
