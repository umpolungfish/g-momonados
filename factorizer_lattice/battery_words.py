import subprocess, re, json, glob
from sympy import factorint
RUN="./run_cmds.sh"

def words_from_raw(N):
    s=open(f"factorizer_lattice/raw_{N}.txt").read()
    hexw=re.search(r"hex-digit word\s*:\s*([⊢⊣≻≺⋈⊙∈∋⊤⊥⊞⊡]+)", s)
    natw=re.search(r"native word\s*:\s*([⊢⊣≻≺⋈⊙∈∋⊤⊥⊞⊡]+)", s)
    return (hexw.group(1) if hexw else None, natw.group(1) if natw else None)

def word_tools(w):
    out=subprocess.run([RUN,f"cycle {w}",f"weight {w}",f"banked {w}",f"trans {w}"],
                       capture_output=True,text=True,timeout=120).stdout
    d={}
    d["cyc_period"]=_i(r"period[:\s]+(\d+)",out)
    d["cyc_cuts"]=_i(r"(\d+)\s+(?:winding-commit|commit|landing|cut)",out)
    d["trans"]=_i(r"(\d+)\s+transition",out)
    d["banked_ok"]= "banked_ok" in out or "bank" in out and "OK" in out
    d["weight_lost"]=_i(r"weight_lost[_a-z ]*[:=]?\s*(\d+)",out)
    d["surplus"]=_i(r"surplus[_a-z ]*[:=]?\s*(\d+)",out)
    d["live_clears"]=_i(r"(\d+)\s+live clear",out)
    d["deposits"]=_i(r"deposits[:=]?\s*(\d+)",out)
    return d,out

def _i(p,s):
    m=re.search(p,s); return int(m.group(1)) if m else None

Ns=[15,21,35,51,77,91,143,155,187,221,247,323,437,667,899,1517,3233,3599,8051,10403]
rows=[]
for N in Ns:
    hexw,natw=words_from_raw(N)
    if not natw: 
        print(N,"no native word"); continue
    dn,rawn=word_tools(natw)
    open(f"factorizer_lattice/words_{N}.txt","w").write("=NATIVE=\n"+rawn)
    f=sorted(factorint(N)); 
    row={"N":N,"nat_len":len(natw),"hex_len":len(hexw) if hexw else None}
    row.update({f"nat_{k}":v for k,v in dn.items()})
    row["facs"]=f
    rows.append(row); print(row,flush=True)
json.dump(rows,open("factorizer_lattice/lattice_words.json","w"),indent=1,default=str)
