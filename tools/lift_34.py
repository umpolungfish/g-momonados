#!/usr/bin/env python3
"""lift_34.py — Stage 34A: the structural signature sigma and the ~s relation.
              Stage 34B: the ⊙-seed / REENTER-void carrier as a surface-pair transposition.

34A: sigma(w) = (paired, substantial, unanswered, substantial_interiors, unmatched_open_positions)
     R1 ~s R2  iff  Δsigma_R1 == Δsigma_R2 over the tested domain.  Sweep every admissible witness.
34B: SurfacePair(word) = (vox_reading, weight_reading).  Probe the transformation H = surface swap.
"""
import subprocess, sys, os, re

GMO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
VOX = os.environ.get("VOX", os.path.join(GMO, "vendor/vox/target/debug/vox"))
sys.path.insert(0, os.path.join(GMO, "tools"))
from lift_reentry import (MORPH, pair, verdict, FSPLIT, FFUSE, TANCH, AFWD, IFIX, EVALT, EVALF)

def vox(*a):
    return subprocess.run([VOX, *a], capture_output=True, text=True).stdout

# ---- sigma: the structural observation ------------------------------------
def sigma(w):
    out = vox("pairs", w)
    rows = re.findall(r"^\s*(\d+)\s+(\d+)\s+(\d+)\s+(yes|no)\s*(.*)$", out, re.M)
    mp = re.search(r"(\d+)\s+paired,\s+(\d+)\s+substantial", out)
    mu = re.search(r"unanswered\s+(\d+)\s+division\(s\)\s+at\s+(\S+)", out)
    mj = re.search(r"unopened\s+(\d+)\s+rejoining\(s\)\s+at\s+(\S+)", out)
    interiors = tuple(r[4].strip() for r in rows if r[3] == "yes")
    return dict(paired=int(mp.group(1)), substantial=int(mp.group(2)),
                unanswered=int(mu.group(1)), unmatched=(mu.group(2) if mu else "-"),
                rejoining=int(mj.group(1)) if mj else 0,
                interiors=interiors, verdict=verdict(w))

def dsigma(w, R):
    a = sigma(w); rw = MORPH[R](w)
    if rw is None:
        return None
    b = sigma(rw)
    return (b["paired"] - a["paired"], b["substantial"] - a["substantial"],
            b["unanswered"] - a["unanswered"],
            (a["interiors"], b["interiors"]), (a["unmatched"], b["unmatched"]))

def fmt(sig):
    if sig is None:
        return "UNDEFINED"
    return f"(dp={sig[0]:+d}, ds={sig[1]:+d}, du={sig[2]:+d}, int={sig[3][0]}->{sig[3][1]}, at={sig[4][0]}->{sig[4][1]})"
# ---- 34A: witness sweep for each morphism class --------------------------
FW_WITNESSES = [
    ("EDIT   ∈∋⊥≻⊡", FSPLIT + FFUSE + EVALF + AFWD + IFIX),
    ("REDUCE ∈∋⊤≻⊡", FSPLIT + FFUSE + EVALT + AFWD + IFIX),
    ("∈∋⊥", FSPLIT + FFUSE + EVALF),
    ("∈∋⊤", FSPLIT + FFUSE + EVALT),
    ("∈∋⊙⊡⊣", FSPLIT + FFUSE + "\u2299" + IFIX + TANCH),
    ("∈∋⋈", FSPLIT + FFUSE + "\u22c8"),
    ("∈∋≻⊤", FSPLIT + FFUSE + AFWD + EVALT),
    ("⊢∈∋⊥≻⊡⊣", "\u22a2" + FSPLIT + FFUSE + EVALF + AFWD + IFIX + TANCH),
    ("two-dyad", FSPLIT + FFUSE + EVALF + AFWD + IFIX + FSPLIT + FFUSE + EVALT + AFWD + IFIX),
]
CF_WITNESSES = [
    ("OP_SHAPE   ⊢∈≻⊤⊣", "\u22a2" + FSPLIT + AFWD + EVALT + TANCH),
    ("⊢∈⊙⊣", "\u22a2" + FSPLIT + "\u2299" + TANCH),
    ("⊢∈⊥⊣", "\u22a2" + FSPLIT + EVALF + TANCH),
    ("∈≻⊤⊣", FSPLIT + AFWD + EVALT + TANCH),
    ("⊢∈≻⊤∈≺∋⊣", "\u22a2" + FSPLIT + AFWD + EVALT + FSPLIT + "\u227a" + FFUSE + TANCH),
]
ID_WITNESSES = [
    ("EDIT type ∈⊥≻⊡∋", FSPLIT + EVALF + AFWD + IFIX + FFUSE),
    ("EDIT walk ∈∋⊥≻⊡", FSPLIT + FFUSE + EVALF + AFWD + IFIX),
    ("OP_PRESERVE ⊢⊣", "\u22a2" + TANCH),
    ("OP_FIX ⊢⊙⊡⊣", "\u22a2" + "\u2299" + IFIX + TANCH),
    ("two-region", FSPLIT + AFWD + EVALF + FFUSE + FSPLIT + AFWD + EVALT + FFUSE),
]

def sweep():
    print("=== Stage 34A: Δσ per morphism class across admissible witnesses ===")
    groups = {"FRAME_WORK": FW_WITNESSES, "CLOSE_FRAME": CF_WITNESSES, "IDENTITY": ID_WITNESSES}
    dsigs = {"FRAME_WORK": [], "CLOSE_FRAME": [], "IDENTITY": []}
    for R, wits in groups.items():
        print(f"-- {R} --")
        for label, w in wits:
            d = dsigma(w, R)
            dsigs[R].append((label, d))
            print(f"   {label:<24} σ={sigma(w)['paired']}p/{sigma(w)['substantial']}s/{sigma(w)['unanswered']}u"
                  f"  ->  {fmt(d)}")
    # invariants?
    print()
    print("-- is Δσ invariant per class (minus the τ-dependent interior)? --")
    for R in groups:
        core = {(d[0], d[1], d[2]) for _, d in dsigs[R] if d is not None}
        print(f"   {R:<12} distinct (dp,ds,du) cores: {sorted(core)}")
    # degeneracy: distinct morphisms with the same full Δσ on shared witnesses
    print()
    print("-- structural degeneracy: τ-dependent cells (dp,ds,du,at) --")
    return dsigs
# ---- 34B: the seed / reenter carrier as a surface-pair transposition ------
SEED = FSPLIT + FFUSE + "\u2299" + AFWD + IFIX          # ∈∋⊙≻⊡
REENTER = "\u22a2" + FSPLIT + "\u227a" + FFUSE + TANCH  # ⊢∈≺∋⊣
T0 = FSPLIT + EVALT + AFWD + IFIX + FFUSE               # ∈⊤≻⊡∋
W0 = FSPLIT + FFUSE + EVALT + AFWD + IFIX               # ∈∋⊤≻⊡
SCAN = AFWD + "\u229e" + "\u22c8" + IFIX                # ≻⊞⋈⊡  (OP_SCAN_APPEND)

def _bits(n):
    return "".join(EVALT if (n >> (7 - k)) & 1 else EVALF for k in range(8))

def clause_transform(C):                                 # C as the applied-word slot
    return FSPLIT + EVALT + "\u2299" + FSPLIT + _bits(len(C)) + FFUSE + C + IFIX + FFUSE

def clause_selector(C):                                  # C as the source slot J
    return FSPLIT + C + "\u2299" + FSPLIT + _bits(4) + FFUSE + "\u22a2\u2299\u22a1\u22a3" + FFUSE

ENVS = [
    ("E1 bare",              lambda C: C),
    ("E2 pair(C,t0)",        lambda C: pair(C, T0)),
    ("E3 pair(w0,C)",        lambda C: pair(W0, C)),
    ("E4 C+FIX",             lambda C: C + IFIX),
    ("E5 C+SCAN_APPEND",     lambda C: C + SCAN),
    ("E6 applied-word slot", clause_transform),
    ("E7 source slot (J)",   clause_selector),
    ("E8 judgment slot (S)", lambda C: FSPLIT + EVALT + C + FSPLIT + _bits(4) + FFUSE + "\u22a2\u2299\u22a1\u22a3" + FFUSE),
]

def run_batch(cmds):
    """One boot: feed every REPL command, return the concatenated stdout."""
    if not cmds:
        return ""
    return subprocess.run([os.path.join(GMO, "run_cmds.sh"), *cmds],
                          capture_output=True, text=True).stdout

def weight_final(out, w):
    # split on the echoed command and read the 'final' line that follows
    seg = out.split("weight " + w, 1)
    if len(seg) < 2:
        return "?"
    m = re.search(r"final\s*:\s*([TFBN])", seg[1])
    return m.group(1) if m else "?"

def s3(out, w):
    seg = out.split("sixteen3 check " + w, 1)
    if len(seg) < 2:
        return "?"
    m = re.search(r"Tri-ancestral verdict:\s*([TFBN])", seg[1])
    return m.group(1) if m else "?"

def surface(words):
    wcmds = ["weight " + w for w in words]
    scmds = ["vox sixteen3 check " + w for w in words]
    wo = run_batch(wcmds)
    so = run_batch(scmds)
    return {w: (verdict(w), weight_final(wo, w), s3(so, w)) for w in words}
# ---- main -----------------------------------------------------------------
def carrier_matrix():
    print("=== Stage 34B: the two carriers in matched environments ===")
    env_words = []
    for label, f in ENVS:
        env_words.append((label, f(SEED), f(REENTER)))
    allw = []
    for _, a, b in env_words:
        allw += [a, b]
    surf = surface(allw)
    print(f"{'environment':<22} {'SEED   (vox,w,S3)':<22} {'REENTER (vox,w,S3)':<22} "
          f"{'P(seed)':<10} {'swap P(re)':<12} {'≡?'}")
    ok_swap = 0
    for label, a, b in env_words:
        va, wa, sa = surf[a]
        vb, wb, sb = surf[b]
        Pa = (va, wa)
        Pb = (vb, wb)
        swap = (Pb[1], Pb[0])
        eq = (Pa == swap)
        ok_swap += eq
        print(f"{label:<22} {str((va,wa,sa)):<22} {str((vb,wb,sb)):<22} "
              f"{str(Pa):<10} {str(swap):<12} {eq}")
    print(f"  H = surface swap commutes on {ok_swap}/{len(env_words)} environments")
    print()
    # projection table: where do the two carriers agree / differ?
    print("-- per-observable agreement (seed vs reenter) --")
    for obs, idx in [("vox", 0), ("weight", 1), ("SIXTEEN_3", 2)]:
        agree = sum(1 for label, a, b in env_words if surf[a][idx] == surf[b][idx])
        print(f"   on {obs:<9}: {agree}/{len(env_words)} environments agree")
    print("  (the two carriers are equivalent on some projections, inequivalent on others — held jointly)")
    print()
    return ok_swap

def main():
    sweep()
    print("STAGE34A: see cores above — a class is invariant iff its (dp,ds,du) core is a singleton.")
    print()
    ok = carrier_matrix()
    print("STAGE34B:", "surface-swap bridge HOLDS" if ok == len(ENVS) else f"swap bridge partial ({ok}/{len(ENVS)})")

if __name__ == "__main__":
    main()
