#!/usr/bin/env python3
"""
STAGE 120 — GLOBAL OVERLAP BUDGET / PAIR-DIFFERENCE IDENTITY

For a finite support A with s=|A|, define positive-lag overlaps

    O_g(A)=|A intersect (A+g)|, g>=1.

Every unordered pair {a,b} with a<b contributes exactly once:
at lag g=b-a.

Therefore

    sum_{g>=1} O_g(A) = C(s,2).

Equivalently, the positive-lag aperiodic autocorrelation has total mass
equal to the number of unordered support pairs.

This means overlap cannot disappear globally; it can only be redistributed
across lags. Choosing an appended weight g selects one coefficient O_g
from this fixed pair-difference budget.

The zero-lag coefficient is
    O_0=s,
and the full bilateral autocorrelation mass is
    s^2 = s + 2*C(s,2).
"""

from itertools import combinations

def overlaps(A):
    if not A:
        return {}
    M=max(A)-min(A)
    return {g:len(A & {a+g for a in A}) for g in range(1,M+1)}

families=[
    {0},
    {0,1},
    {0,1,2,3},
    {0,2,5,9},
    {0,1,3,4,10},
    set(range(16)),
    {0,1,2,4,7,11,18},
]
for A in families:
    s=len(A)
    os=overlaps(A)
    assert sum(os.values()) == s*(s-1)//2

    # Difference histogram audit.
    diffs={}
    for a,b in combinations(sorted(A),2):
        d=b-a
        diffs[d]=diffs.get(d,0)+1
    assert os=={g:v for g,v in diffs.items()} | {
        g:0 for g in range(1,(max(A)-min(A))+1) if g not in diffs
    } if len(A)>1 else os=={}

print("STAGE 120 — GLOBAL OVERLAP BUDGET / PAIR-DIFFERENCE IDENTITY")
print("="*94)
print("120A positive-lag overlap:")
print("  O_g(A)=|A intersect (A+g)|")
print()
print("120B pair-difference theorem:")
print("  sum_{g>=1} O_g(A)=C(|A|,2)")
print()
print("120C meaning:")
print("  every unordered support pair contributes once,")
print("  at its unique positive difference g=b-a.")
print()
print("120D autocorrelation mass:")
print("  O_0=|A|")
print("  bilateral total = |A|^2")
print()
print("120E insertion consequence:")
print("  choosing a new weight g samples one coefficient O_g")
print("  from the fixed positive-lag pair-difference budget.")
print()
print("120F finite audit:")
print("  interval, sparse, and irregular supports: TRUE")
print()
print("PARACONSISTENT LANDING")
print("  local overlap at a chosen lag can vanish")
print("  AND the global positive-lag overlap budget remains C(s,2).")
print("STAGE 120 RESULT : True")
