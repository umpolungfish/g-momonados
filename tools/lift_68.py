#!/usr/bin/env python3
"""
STAGE 68 — ROOTED CLOSURE LANGUAGE / PHASE-SENSITIVE MACROCYCLE

Grounded from the measured 20 ordered pair polymerizations of:
  C = coupling_algebra
  M = minimal_cover
  S = stage_67
  U = union_semilattice
  R = mobius_reconstruction

Pair types:
  0 = no reaction center / termination
  1 = clean condensation (macrocyclizable as a closing edge)
  X = two-center cross-link (can occur internally, but measured as telechelic
      when used as a 2-body head-to-tail closure)

The measured pair graph is symmetric:
  missing: C-S, M-R
  cross:   M-U, U-R
  clean:   all remaining unordered pairs

This script checks the exact closure language induced by those measured pair
rules, its C5/D5 structure, and the exact spectrum of the best measured
macrocycle (a 5-cycle with one doubled cross-link edge).
"""

from itertools import permutations
from collections import Counter
import sympy as sp

NODES = ("C", "M", "S", "U", "R")

# unordered measured pair type
PAIR = {
    frozenset(("C","M")): "1",
    frozenset(("C","S")): "0",
    frozenset(("C","U")): "1",
    frozenset(("C","R")): "1",
    frozenset(("M","S")): "1",
    frozenset(("M","U")): "X",
    frozenset(("M","R")): "0",
    frozenset(("S","U")): "1",
    frozenset(("S","R")): "1",
    frozenset(("U","R")): "X",
}

def pair_type(a, b):
    if a == b:
        return "-"
    return PAIR[frozenset((a,b))]

def bonded(a, b):
    return pair_type(a,b) in ("1","X")

def macrocyclizable(a, b):
    return pair_type(a,b) == "1"

def classify(word):
    """Measured pair-local grammar for a rooted 5-word."""
    # first four are sequential polymerization bonds
    if not all(bonded(word[i], word[i+1]) for i in range(4)):
        return "internal_block"
    # last->first is the head-to-tail cyclization edge
    t = pair_type(word[-1], word[0])
    if t == "0":
        return "final_missing"
    if t == "X":
        return "final_cross"
    return "close"

def rotations(word):
    return [word[i:]+word[:i] for i in range(len(word))]

def d5_orbit(word):
    out = set()
    for q in (word, tuple(reversed(word))):
        out.update(rotations(q))
    return out

def canonical_cycle(word):
    return min(d5_orbit(word))

def edge_signature(cyc):
    return "".join(pair_type(cyc[i], cyc[(i+1)%5]) for i in range(5))

all_words = list(permutations(NODES))
classes = Counter(classify(w) for w in all_words)
assert classes == Counter({
    "close": 32,
    "final_cross": 8,
    "final_missing": 8,
    "internal_block": 72,
})

# Hamiltonian words in the total bond graph (clean + cross)
ham = [w for w in all_words if all(bonded(w[i],w[(i+1)%5]) for i in range(5))]
assert len(ham) == 40

# Unrooted Hamiltonian cycles modulo D5
hcycles = {}
for w in ham:
    c = canonical_cycle(w)
    hcycles[c] = edge_signature(c)
assert len(hcycles) == 4
assert all(sig.count("X") == 1 and sig.count("1") == 4 for sig in hcycles.values())

# Reversal/chirality symmetry of closure language
assert all(classify(w) == classify(tuple(reversed(w))) for w in all_words)

# D5 orbit census
seen = set()
orbit_rows = []
for w in all_words:
    if w in seen:
        continue
    orb = d5_orbit(w)
    seen |= orb
    c = min(orb)
    orbit_rows.append((c, edge_signature(c), Counter(classify(q) for q in orb)))
assert len(orbit_rows) == 12

# Every Hamiltonian orientation has a 5-rotation signature containing
# four closes and one final_cross.
oriented_reps = []
seen_oriented = set()
for w in ham:
    ro = frozenset(rotations(w))
    if ro in seen_oriented:
        continue
    seen_oriented.add(ro)
    oriented_reps.append(w)
assert len(oriented_reps) == 8
rot_sigs = []
for w in oriented_reps:
    sig = tuple(1 if classify(q) == "close" else 0 for q in rotations(w))
    assert sum(sig) == 4
    assert Counter(classify(q) for q in rotations(w)) == Counter({"close":4, "final_cross":1})
    rot_sigs.append(sig)

# Exact spectrum of the best measured ring:
# C-M-S-U-R-C with U-R doubled (cross-link has two reaction centers).
lam = sp.symbols("lam")
A = sp.Matrix([
    [0,1,0,0,1],
    [1,0,1,0,0],
    [0,1,0,1,0],
    [0,0,1,0,2],
    [1,0,0,2,0],
])
charpoly = sp.factor(A.charpoly(lam).as_expr())
expected_cp = (lam-1)*(lam**2-lam-4)*(lam**2+2*lam-1)
assert sp.expand(charpoly - expected_cp) == 0

rho = (1 + sp.sqrt(17))/2
gap = sp.simplify(rho - (1 + sp.sqrt(2)))
energy = sp.simplify(1 + sp.sqrt(17) + 2*sp.sqrt(2))

print("STAGE 68 — ROOTED CLOSURE LANGUAGE / PHASE-SENSITIVE MACROCYCLE")
print("="*76)
print("68A full measured pair graph:")
print("     C M S U R")
for a in NODES:
    print(f"  {a}: " + " ".join(pair_type(a,b) for b in NODES))
print("  legend: 0=no bond, 1=clean condensation, X=two-center cross-link")
print()

print("68B closure-language partition over S5: TRUE")
print(f"  close          = {classes['close']}")
print(f"  final_cross    = {classes['final_cross']}")
print(f"  final_missing  = {classes['final_missing']}")
print(f"  internal_block = {classes['internal_block']}")
print("  total          =", sum(classes.values()))
print()

print("68C Hamiltonian comparison: TRUE")
print("  bond-graph Hamiltonian rooted words =", len(ham))
print("  actual closing rooted words         =", classes["close"])
print("  overprediction                      =", len(ham)-classes["close"])
print("  exact cause: the 8 overpredictions place a two-center cross-link")
print("               on the final head-to-tail cyclization edge.")
print()

print("68D unrooted/D5 structure: TRUE")
print("  D5 orbits in S5 =", len(orbit_rows))
print("  unrooted Hamiltonian cycles =", len(hcycles))
for c,sig in sorted(hcycles.items()):
    print("   ", "-".join(c), "edge-types", sig)
print("  each Hamiltonian D5 orbit: 8 close + 2 final_cross")
print("  reversal symmetry: c(w)=c(reverse(w)) for all 120 words")
print()

print("68E cyclic-shift signatures: TRUE")
for i,(w,sig) in enumerate(zip(oriented_reps,rot_sigs),1):
    print(f"  orbit {i}: {'-'.join(w)} -> {sig}")
print("  every oriented Hamiltonian cycle has four closing roots and one")
print("  nonclosing root: the cut that puts the cross-link at head-to-tail.")
print()

print("68F exact macrocycle spectrum: TRUE")
print("  characteristic polynomial =", sp.factor(charpoly))
print("  spectrum = {1, (1±sqrt(17))/2, -1±sqrt(2)}")
print("  rho =", rho, "≈", float(rho))
print("  gap =", gap, "≈", float(gap))
print("  energy =", energy, "≈", float(energy))
print()

print("PARACONSISTENT LANDING")
print("  the compatibility graph is pair-local AND rooted closure is phase-sensitive")
print("  cross-links are valid internal bonds AND invalid head-to-tail closure edges")
print("  reversal is a symmetry AND cyclic rotation is not a closure symmetry")
print("  the ring is a 5-cycle AND one doubled edge makes its spectrum branched")
print()
print("STAGE 68 RESULT : True")
