import subprocess, re, json
from sympy import factorint
RUN="./run_cmds.sh"
def hexword(N):
    s=open(f"factorizer_lattice/raw_{N}.txt").read()
    m=re.search(r"hex-digit word\s*:\s*([⊢⊣≻≺⋈⊙∈∋⊤⊥⊞⊡]+)",s); return m.group(1) if m else None
def vox_regions(w):
    out=subprocess.run(["../Vox/vox","pairs",w],capture_output=True,text=True,timeout=60).stdout
    return re.findall(r"^\s*(\d+)\s+(\d+)\s+(\d+)\s+(?:yes|no)\s+(\S+)",out,re.M)  # open,close,span,interior
Ns=[143,221,3233,667,10403,1517,187,3599,155]
for N in Ns:
    w=hexword(N); regs=vox_regions(w); f=sorted(factorint(N))
    # relative phase = gap between the two region OPEN positions, and span pair
    if len(regs)>=2:
        o=[int(r[0]) for r in regs]; c=[int(r[1]) for r in regs]; sp=[int(r[2]) for r in regs]
        gap=o[1]-o[0]
        p,q=f if len(f)==2 else (f[0],f[0])
        print(f"N={N} f={f}  opens={o} closes={c} spans={sp}  gap={gap}  q-p={q-p} (q-p)/2={(q-p)//2} p={p} q={q}  gap*?={gap}  close_gap={c[1]-c[0]}")
    else:
        print(f"N={N} f={f}  <2 regions: {[(r[0],r[1],r[2]) for r in regs]}")
