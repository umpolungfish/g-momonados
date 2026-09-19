#!/usr/bin/env python3
"""
STAGE 106 — CANONICAL WEIGHT NORMAL FORM / CONFLUENT REWRITE

Abstract free-word lift on generators z1,z2,z3,z4.

Rewrite rules:
    z2 -> z1 z1
    z3 -> z1 z1 z1
    z4 -> z1 z1 z1 z1

Properties:
1. Weight preserving.
2. Terminating: every rewrite strictly decreases the number of
   non-z1 generators.
3. Confluent: each generator rewrites independently to a unique z1-string.
4. Unique normal form:
       NF(w) = z1^{wt(w)}.
5. Therefore:
       u ~_tau v  iff  NF(u)=NF(v).

This constructs the quotient normal form explicitly.

Qualification:
  This is a mathematical rewrite system on the abstract resolved-word lift.
  It is not asserted as a native IMASM rewrite rule.
"""

from itertools import product

gens = {"z1":1, "z2":2, "z3":3, "z4":4}

def wt(w):
    return sum(gens[z] for z in w)

def nf(w):
    return tuple("z1" for _ in range(wt(w)))

def nonunit_count(w):
    return sum(z != "z1" for z in w)

def one_step_variants(w):
    out = []
    for i,z in enumerate(w):
        g = gens[z]
        if g > 1:
            repl = ("z1",)*g
            out.append(w[:i] + repl + w[i+1:])
    return out

# Weight preservation + strict termination measure.
alphabet = tuple(gens)
sample = [()]
for n in range(1,5):
    sample += list(product(alphabet, repeat=n))

for w in sample:
    for v in one_step_variants(w):
        assert wt(v) == wt(w)
        assert nonunit_count(v) < nonunit_count(w)

# Exhaustively explore all rewrite paths for short words and confirm
# unique normal form.
def normal_forms(w):
    seen = set()
    finals = set()
    stack = [w]
    while stack:
        u = stack.pop()
        if u in seen:
            continue
        seen.add(u)
        nxt = one_step_variants(u)
        if not nxt:
            finals.add(u)
        else:
            stack.extend(nxt)
    return finals

for w in sample:
    finals = normal_forms(w)
    assert finals == {nf(w)}

# Kernel iff normal-form equality.
for u in sample[:300]:
    for v in sample[:300]:
        assert (wt(u) == wt(v)) == (nf(u) == nf(v))

# Native Stage-102/103 schedule subset.
bits_all = list(product((0,1), repeat=4))
names = ("z1","z2","z3","z4")
schedule_words = [
    tuple(z for b,z in zip(bits,names) if b)
    for bits in bits_all
]
for w in schedule_words:
    assert nf(w) == ("z1",)*wt(w)

print("STAGE 106 — CANONICAL WEIGHT NORMAL FORM / CONFLUENT REWRITE")
print("="*88)
print("106A rewrite system:")
print("  z2 -> z1 z1")
print("  z3 -> z1 z1 z1")
print("  z4 -> z1 z1 z1 z1")
print()
print("106B weight preservation: TRUE")
print("  every rewrite preserves wt")
print()
print("106C termination: TRUE")
print("  each rewrite strictly decreases the number of non-z1 generators")
print()
print("106D confluence / unique normal form: TRUE")
print("  exhaustive audit for all words of length <=4")
print()
print("106E canonical representative:")
print("  NF(w)=z1^wt(w)")
print()
print("106F kernel characterization:")
print("  u ~_tau v  iff  NF(u)=NF(v)")
print()
print("106G native schedule restriction:")
print("  all 16 Stage-102/103 schedule words normalize by their measured")
print("  aggregate weight class.")
print()
print("106H qualification:")
print("  this rewrite system lives in the abstract word lift;")
print("  it is not claimed as a native IMASM rewrite rule.")
print()
print("PARACONSISTENT LANDING")
print("  distinct resolved words can share one canonical quotient form")
print("  AND their pre-normalization distinction remains available upstream.")
print()
print("STAGE 106 RESULT : True")
