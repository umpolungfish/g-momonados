#!/usr/bin/env python3
"""lift_reentry.py — Stage 31: LIFT as a re-entry object; Stage 32: is the morphism family
{FRAME_WORK, CLOSE_FRAME, IDENTITY} closed under lifting?

LIFT(R, (w,t)) = (R(w), t) if R applies else (w,t)
walk_L = pair(w,t)   (pre-lift pair-wire; carries the defective walk projection)
type_L = pair(t,t)   (post-lift pair-wire; the diagonal)

The Router's CLOSE_FRAME is CLAUSE-AWARE (Stage 27): it closes the unmatched FSPLIT inside the
embedded transform `⊢∈≻⊤⊣` and bumps that clause's 8-bit transform-length field, reproducing the
resident 181-mark ladder byte-for-byte.
"""
import subprocess, sys, os, re

GMO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
VOX = os.environ.get("VOX", os.path.join(GMO, "vendor/vox/target/debug/vox"))
TW  = os.environ.get("TW",  os.path.join(GMO, "Vox/target/debug/tower_words_cli"))

FSPLIT, FFUSE, TANCH, AFWD, IFIX, EVALT, EVALF = "\u2208", "\u220b", "\u22a3", "\u227b", "\u22a1", "\u22a4", "\u22a5"

def vox(*a):
    return subprocess.run([VOX, *a], capture_output=True, text=True).stdout

def verdict(w):
    m = re.search(r"verdict\s+([TFBN])", vox("verdict", w))
    return m.group(1) if m else "?"

def pairstats(w):
    out = vox("pairs", w)
    m = re.search(r"^\s*(\d+)\s+(\d+)\s+(\d+)\s+(yes|no)\s*(.*)$", out, re.M)
    r = re.search(r"(\d+)\s+paired,\s+(\d+)\s+substantial", out)
    u = re.search(r"unanswered\s+(\d+)", out)
    if not m:
        return dict(span=0, work=False, paired=0, substantial=0, unanswered=0)
    return dict(span=int(m.group(3)), work=m.group(4) == "yes",
                paired=int(r.group(1)) if r else 0,
                substantial=int(r.group(2)) if r else 0,
                unanswered=int(u.group(1)) if u else -1)

def numeral(n):
    return vox("numeral", str(n)).strip()

def pair(walk, typ):
    return f"{FSPLIT}{numeral(len(walk))}{FFUSE}{walk}{FSPLIT}{numeral(len(typ))}{FFUSE}{typ}"

def _stack_unmatched(w, lo, hi):
    op = []
    for k in range(lo, hi):
        if w[k] == FSPLIT:
            op.append(k)
        elif w[k] == FFUSE and op:
            op.pop()
    return op

# ---- partial word morphisms ----------------------------------------------
def frame_work(w):
    i = w.find(FSPLIT + FFUSE)
    return None if i < 0 else w[:i] + FSPLIT + w[i + 2:] + FFUSE

def parse_clauses(w):
    """Parse `∈ J S ∈ <8 bits> ∋ <L marks> <next> ∋` clauses; return (list, consumed)."""
    out = []
    i, n = 0, len(w)
    while i + 15 <= n:
        if not (w[i] == FSPLIT and w[i + 3] == FSPLIT and w[i + 12] == FFUSE):
            break
        bits = w[i + 4:i + 12]
        if any(b not in (EVALT, EVALF) for b in bits):
            break
        L = int("".join("1" if b == EVALT else "0" for b in bits), 2)
        ts, te = i + 13, i + 13 + L
        if te + 2 > n or w[te + 1] != FFUSE:
            break
        out.append((i, bits, ts, te))
        i = te + 2
    return out, i

def close_frame(w):
    cls, _ = parse_clauses(w)
    for (i, bits, ts, te) in cls:
        op = _stack_unmatched(w, ts, te)          # unmatched FSPLIT inside this transform
        if op:
            p = op[0]
            j = w.find(TANCH, p + 1)
            j = len(w) if j < 0 else j
            L = int("".join("1" if b == EVALT else "0" for b in bits), 2) + 1
            nb = "".join(EVALT if (L >> (7 - k)) & 1 else EVALF for k in range(8))
            w2 = w[:i + 4] + nb + w[i + 12:]                     # bump the clause length field
            return w2[:j] + FFUSE + w2[j:]                        # close the transform's ∈
    op = _stack_unmatched(w, 0, len(w))                        # bare-word fallback
    if not op:
        return None
    p = op[0]
    j = w.find(TANCH, p + 1)
    j = len(w) if j < 0 else j
    return w[:j] + FFUSE + w[j:]

def identity(w):
    return w

MORPH = {"FRAME_WORK": frame_work, "CLOSE_FRAME": close_frame, "IDENTITY": identity}

def apply_lift(R, walk, typ):
    w2 = MORPH[R](walk)
    return (walk, typ) if w2 is None else (w2, typ)

# ---- tower rows -----------------------------------------------------------
def tower():
    tw = {}
    for line in subprocess.run([TW], capture_output=True, text=True).stdout.splitlines():
        p = line.split("\t")
        if len(p) >= 3:
            tw[p[0]] = p[2]
    return tw

def rows():
    tw = tower()
    type_R = open("/tmp/rp.txt").read().strip()
    return [
        ("Edit",     FSPLIT + FFUSE + EVALF + AFWD + IFIX, FSPLIT + EVALF + AFWD + IFIX + FFUSE, "FRAME_WORK"),
        ("Schedule", FSPLIT + FFUSE + EVALT + AFWD + IFIX, FSPLIT + EVALT + AFWD + IFIX + FFUSE, "FRAME_WORK"),
        ("Router",   tw["ROUTER"], type_R, "CLOSE_FRAME"),
        ("Trace",    tw["TRACE"], tw["TRACE"], "IDENTITY"),
        ("History",  tw["HISTORY"], tw["HISTORY"], "IDENTITY"),
    ]

def stage31():
    print("=== Stage 31: LIFT as a re-entry object (walk_L, type_L) ===")
    walkR = tower()["ROUTER"]; rp = open("/tmp/rp.txt").read().strip()
    cls, end = parse_clauses(walkR)
    print(f"clause parse: {len(cls)} clauses, consumed {end}/{len(walkR)}")
    print("CLOSE_FRAME(walk_Router) == resident repair(181):", close_frame(walkR) == rp)
    print(f"{'row':<9} {'R':<11} {'R(w)==t':<9} {'v(walk_L)':<9} {'v(type_L)':<9} "
          f"{'LIFT(walk_L)==type_L':<20} {'LIFT(type_L)==type_L':<20}")
    ok = True
    for name, w, t, R in rows():
        wl, tl = pair(w, t), pair(t, t)
        c_rw = (MORPH[R](w) == t)
        c_lift = (pair(*apply_lift(R, w, t)) == tl)
        c_fix = (pair(*apply_lift(R, t, t)) == tl)
        ok &= c_rw and c_lift and c_fix
        print(f"{name:<9} {R:<11} {str(c_rw):<9} {verdict(wl):<9} {verdict(tl):<9} "
              f"{str(c_lift):<20} {str(c_fix):<20}")
    print("STAGE31", "PASS" if ok else "FAIL")
    print(f"{'row':<9} {'walk_L':<24} {'type_L':<24}")
    for name, w, t, R in rows():
        wl, tl = pair(w, t), pair(t, t)
        pw, pt = pairstats(wl), pairstats(tl)
        print(f"{name:<9} {verdict(wl):<2} {pw['paired']}p/{pw['substantial']}s u{pw['unanswered']:<11} "
              f"{verdict(tl):<2} {pt['paired']}p/{pt['substantial']}s u{pt['unanswered']}")
    print()
    return ok

# ---- Stage 32: closure under lifting -------------------------------------
def effective(R):
    def f(w):
        x = MORPH[R](w)
        return w if x is None else x
    return f

def stage32():
    print("=== Stage 32: is {FRAME_WORK, CLOSE_FRAME, IDENTITY} closed under lifting? ===")
    wires = {
        "w1=∈∋⊥≻⊡": FSPLIT + FFUSE + EVALF + AFWD + IFIX,
        "w2=∈∋⊤≻⊡": FSPLIT + FFUSE + EVALT + AFWD + IFIX,
        "w3=⊢∈≻⊤⊣": "\u22a2" + FSPLIT + AFWD + EVALT + TANCH,
        "w4=∈⊤≻⊡∋": FSPLIT + EVALT + AFWD + IFIX + FFUSE,
    }
    names = list(wires)
    print("-- raw morphisms (partial): None = undefined; '->n' = length of image --")
    print(f"{'wire':<14} " + " ".join(f"{n:<11}" for n in ("FRAME_WORK", "CLOSE_FRAME", "IDENTITY")))
    for wn in names:
        w = wires[wn]
        cells = []
        for R in ("FRAME_WORK", "CLOSE_FRAME", "IDENTITY"):
            x = MORPH[R](w)
            cells.append("~" if x is None else ("id" if x == w else f"->{len(x)}"))
        print(f"{wn:<14} " + " ".join(f"{c:<11}" for c in cells))

    print()
    print("-- family signature under LIFT (totalized): idempotence at the walk slot --")
    ok_idem = True
    for R in ("FRAME_WORK", "CLOSE_FRAME", "IDENTITY"):
        f = effective(R)
        idem = all(f(f(w)) == f(w) for w in wires.values())
        ok_idem &= idem
        print(f"  LIFT({R:<11}) idempotent: {idem}")
    print("CLOSURE-IDEMPOTENT", "PASS" if ok_idem else "FAIL")

    print()
    print("-- functoriality at the walk slot: LIFT(R2∘R1) vs LIFT(R2)∘LIFT(R1) --")
    fam = ("FRAME_WORK", "CLOSE_FRAME", "IDENTITY")
    ok_func = True
    for R1 in fam:
        for R2 in fam:
            def L(w, R1=R1, R2=R2):
                x = MORPH[R1](w); x = w if x is None else x
                y = MORPH[R2](x); return x if y is None else y
            def LC(w, R1=R1, R2=R2):
                z = MORPH[R2](MORPH[R1](w)) if MORPH[R1](w) is not None else None
                return w if z is None else z
            agree = all(L(w) == LC(w) for w in wires.values())
            ok_func &= agree
            tag = "" if agree else "   <-- totalization gap"
            print(f"  LIFT({R2:<11})∘LIFT({R1:<11}) == LIFT({R2}∘{R1}) : {agree}{tag}")
    print("CLOSURE-FUNCTORIAL", "PASS" if ok_func else "FAIL (totalization adds identity where R1 is undefined)")

    print()
    print("-- injectivity: lifted family size on the test wires --")
    distinct = len({tuple(effective(R)(w) for w in wires.values()) for R in fam})
    print(f"  distinct lifted morphisms: {distinct} / {len(fam)}")
    print("CLOSURE-INJECTIVE", "PASS" if distinct == len(fam) else "FAIL")

    print()
    print("-- verdict --")
    print("  set-closed  : NO  — LIFT(R) acts on PAIRS; the image lives one tower level up.")
    print("  signature   : YES — each lifted morphism is idempotent with the diagonal its fixed point.")
    print("  functorial  : only on the common domain; totalization adds identity outside it.")
    return ok_idem, ok_func, distinct

if __name__ == "__main__":
    ok = stage31()
    stage32()
    sys.exit(0 if ok else 1)
