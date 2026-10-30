import subprocess,re,json
from sympy import factorint
RUN="./run_cmds.sh"
def hexword(N):
    s=open(f"factorizer_lattice/raw_{N}.txt").read()
    m=re.search(r"hex-digit word\s*:\s*([⊢⊣≻≺⋈⊙∈∋⊤⊥⊞⊡]+)",s)
    return m.group(1) if m else None
def cyc(w):
    out=subprocess.run([RUN,f"cycle {w}"],capture_output=True,text=True,timeout=120).stdout
    # rows: k  final  verdict  word
    landings=re.findall(r"^\s*(\d+)\s+([A-Za-z]+)\s+([TBNF])\s",out,re.M)
    period=re.search(r"period (\d+)",out)
    regs=[l[1] for l in landings]
    verds=[l[2] for l in landings]
    # commit cuts = positions where verdict flips to T from non-T, or register lands special
    from collections import Counter
    return {"period":int(period.group(1)) if period else None,
            "reg_counts":dict(Counter(regs)),
            "verd_counts":dict(Counter(verds)),
            "n_land":len(landings)}
Ns=[15,21,35,51,77,91,143,155,187,221,247,323,437,667,899,1517,3233,3599,8051,10403]
rows=[]
for N in Ns:
    w=hexword(N)
    if not w: continue
    d=cyc(w); f=sorted(factorint(N))
    row={"N":N,"hexlen":len(w),**d,"facs":f}
    rows.append(row); print(row,flush=True)
json.dump(rows,open("factorizer_lattice/lattice_hex.json","w"),indent=1,default=str)
