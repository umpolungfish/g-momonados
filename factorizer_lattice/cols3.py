import re, json
from sympy import factorint
def natword(N):
    s=open(f"factorizer_lattice/raw_{N}.txt").read()
    m=re.search(r"native word\s*:\s*([⊢⊣≻≺⋈⊙∈∋⊤⊥⊞⊡]+)",s); return m.group(1) if m else None
def bits(w):  # ⊥=1, ⊤=0, LSB-first per the cell format ≻⋈∈bit∋
    return [1 if c=='⊥' else 0 for i,c in enumerate(w) if c in '⊥⊤']
def show(N):
    w=natword(N); b=bits(w); f=sorted(factorint(N))
    n=len(b)
    print(f"### N={N} f={f} nbits={n} nbits%3={n%3}")
    # arrange in rows of 3 -> columns of length ceil(n/3)
    import math
    ncol=3
    rows=[b[i:i+3] for i in range(0,n,3)]
    # columns
    cols=[[r[c] for r in rows if c<len(r)] for c in range(3)]
    colvals=[sum(v<<i for i,v in enumerate(c)) for c in cols]   # LSB-first per column
    colvalsM=[sum(v<<i for i,v in enumerate(reversed(c))) for c in cols]
    print("  col bitstrings:", [''.join(map(str,c)) for c in cols])
    print("  col values LSB:", colvals, " MSB:", colvalsM)
    from math import gcd
    hits=[(v,gcd(v,N)) for v in colvals+colvalsM if 1<gcd(v,N)<N]
    print("  gcd hits:", hits, " | p,q=",f)
for N in [143,221,3233,667,10403,899,1517,8051,3599,155]:
    show(N)
