#!/usr/bin/env python3
"""lift_35.py - Stage 35.

35A: refine CLOSE_FRAME's core.  Stage 34A fixed (dp,du)=(+1,-1) but ds split
     {0,+1}.  Find an ORTHOGONAL measured observable kappa that separates the
     ds=0 witness (empty interior) from the ds=+1 witnesses -> CLOSE_FRAME's
     single cell becomes a 2-cell B-region.
35B: does H = surface-swap extend to a 3rd surface (SIXTEEN_3)?  Test every
     permutation of the 3-surface reading on the two breaking environments,
     and test the degeneracy SIXTEEN_3 == vox reading-by-reading.
"""
import subprocess, sys, os, re, itertools
GMO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(GMO, "tools"))
import lift_34 as L
from lift_reentry import MORPH

def run_batch(cmds):
    if not cmds:
        return ""
    return subprocess.run([os.path.join(GMO, "run_cmds.sh"), *cmds],
                          capture_output=True, text=True).stdout

def weight_body(out, w):
    m = re.search(r"weight\s+" + re.escape(w) + r"\s*\n", out)
    if not m:
        return None
    body = out[m.end():]
    nxt = re.search(r">\s*weight\s", body)
    if nxt:
        body = body[:nxt.start()]
    return body

def weight_fields(out, w):
    """Parse the weight summary: 'deposits N cleared N restored N seeded N inert N'
    (space-separated, no colon) plus 'surviving: X' and 'final : X'."""
    body = weight_body(out, w)
    if body is None:
        return {}
    d = {}
    for m in re.finditer(r"(deposits|cleared|restored|seeded|inert)\s+(\d+)", body):
        d[m.group(1)] = int(m.group(2))
    ms = re.search(r"surviving\s*:\s*(\S.*?)\s*$", body, re.M)
    d["surviving"] = ms.group(1) if ms else None
    mf = re.search(r"final\s*:\s*(\S+)", body)
    d["final"] = mf.group(1) if mf else None
    return d

# ---- 35A: a kappa observable that separates ds=0 from ds=+1 ----------------
def refine_close_frame():
    print("=== Stage 35A: an observable separating CLOSE_FRAME ds=0 vs ds=+1 ===")
    pairs = [(label, w, MORPH["CLOSE_FRAME"](w)) for label, w in L.CF_WITNESSES]
    allw = []
    for _, a, b in pairs:
        allw += [b]
    out = run_batch(["weight " + w for w in allw])
    print(f"{'witness':<20} {'(dp,ds,du)':<13} {'deposits':<9} {'surviving':<12} {'final'}")
    rows = []
    for label, w, w2 in pairs:
        d = L.dsigma(w, "CLOSE_FRAME")
        core = (d[0], d[1], d[2]) if d else None
        f = weight_fields(out, w2)
        rows.append((label, core, f))
        print(f"{label:<20} {str(core):<13} {str(f.get('deposits')):<9} "
              f"{str(f.get('surviving')):<12} {f.get('final')}")
    print()
    print("-- candidate kappa fields: ds=0 witnesses vs ds=+1 witnesses --")
    ds0 = [r for r in rows if r[1] and r[1][1] == 0]
    ds1 = [r for r in rows if r[1] and r[1][1] == 1]
    for fld in ["deposits", "surviving", "final"]:
        v0 = {r[2].get(fld) for r in ds0}
        v1 = {r[2].get(fld) for r in ds1}
        sep = bool(v0) and bool(v1) and v0.isdisjoint(v1)
        print(f"   {fld:<10} ds=0 -> {sorted(map(str,v0))}   ds=+1 -> {sorted(map(str,v1))}   "
              f"{'SEPARATES' if sep else '-'}")
    return rows

# ---- 35B: extend H to a 3rd surface (SIXTEEN_3) ---------------------------
PERMS = list(itertools.permutations(range(3)))

def perm3(t, p):
    return tuple(t[i] for i in p)

def extend_surface():
    print("=== Stage 35B: H = surface-swap extended to the 3-surface reading ===")
    env_words = [(label, f(L.SEED), f(L.REENTER)) for label, f in L.ENVS]
    allw = []
    for _, a, b in env_words:
        allw += [a, b]
    surf = L.surface(allw)
    print(f"{'environment':<22} {'SEED (vox,w,S3)':<20} {'REENTER (vox,w,S3)':<20} perms: SEED->REENTER")
    per_env = {}
    for label, a, b in env_words:
        ta = surf[a]; tb = surf[b]
        good = [p for p in PERMS if perm3(ta, p) == tb]
        per_env[label] = good
        names = ["".join("vws"[i] for i in p) for p in good]
        print(f"{label:<22} {str(ta):<20} {str(tb):<20} {names}")
    print()
    print("-- does one permutation map SEED -> REENTER on ALL environments? --")
    for p in PERMS:
        n = sum(1 for label in per_env if p in per_env[label])
        print(f"   perm {''.join('vws'[i] for i in p)}: works on {n}/{len(per_env)}")
    print()
    # the degeneracy that explains it: is SIXTEEN_3 a copy of the vox face?
    same = sum(1 for w in allw if surf[w][0] == surf[w][2])
    diff = [(w, surf[w]) for w in allw if surf[w][0] != surf[w][2]]
    print(f"-- degeneracy check: SIXTEEN_3 == vox on {same}/{len(allw)} readings --")
    for w, t in diff:
        print(f"   DIFFERS  {w}  ->  (vox,w,S3)={t}")
    print()
    for br in ["E5 C+SCAN_APPEND", "E6 applied-word slot"]:
        good = per_env.get(br, [])
        print(f"   breaker {br:<22} working permutations: "
              f"{[''.join('vws'[i] for i in p) for p in good]}")
    return per_env, same, len(allw)

def main():
    refine_close_frame()
    print("STAGE35A: ds=0 vs ds=+1 is split by the field marked SEPARATES above.")
    print()
    per_env, same, tot = extend_surface()
    print("STAGE35B: no 3-surface permutation maps SEED->REENTER on a breaking env; "
          f"SIXTEEN_3 == vox on {same}/{tot} readings.")
    if same == tot:
        print("          => the 3rd surface is DEGENERATE with the vox face; H has no "
              "independent third axis to extend to (2-surface swap stands).")

if __name__ == "__main__":
    main()
