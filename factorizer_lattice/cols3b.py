import subprocess,re,json
from sympy import factorint
from math import gcd
RUN="./run_cmds.sh"
def hexword(N):
    s=open(f"factorizer_lattice/raw_{N}.txt").read()
    m=re.search(r"hex-digit word\s*:\s*([⊢⊣≻≺⋈⊙∈∋⊤⊥⊞⊡]+)",s); return m.group(1) if m else None
def landings(w):
    out=subprocess.run([RUN,f"cycle {w}"],capture_output=True,text=True,timeout=120).stdout
    return [m[1] for m in re.findall(r"^\s*(\d+)\s+([A-Za-z]+)\s+[TBNF]\s",out,re.M)]
def hasT(st): return 1 if 'T' in st else 0
def hasF(st): return 1 if 'F' in st else 0
def cols(bits):
    rows=[bits[i:i+3] for i in range(0,len(bits),3)]
    return [[r[c] for r in rows if c<len(r)] for c in range(3)]
for N in [143,221,3233,667,10403,899,1517,8051,3599,155,247,187]:
    w=hexword(N); seq=landings(w); f=sorted(factorint(N))
    Tb=[hasT(s) for s in seq]; Fb=[hasF(s) for s in seq]
    for name,bits in (("T",Tb),("F",Fb)):
        cs=cols(bits)
        vals=[sum(v<<i for i,v in enumerate(c)) for c in cs]
        valsM=[sum(v<<i for i,v in enumerate(reversed(c))) for c in cs]
        hits=[(v,gcd(v,N)) for v in vals+valsM if 1<gcd(v,N)<N]
        cstr=[''.join(map(str,c)) for c in cs]
        print(f"N={N} f={f} [{name}] cols={cstr} vals={vals}/{valsM} gcd={hits}")
