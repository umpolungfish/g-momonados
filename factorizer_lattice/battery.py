import subprocess, re, json, sys
from sympy import factorint
from math import gcd, lcm, isqrt

RUN = "./run_cmds.sh"
# tools that consume a single number N
CMDS = lambda N: [
    f"trilattice_factor read {N}",
    f"trilattice_factor squares {N}",
    f"trilattice_factor winding {N}",
    f"trilattice_factor bridge {N}",
    f"oneshot_prime_winder {N}",
    f"dyn_nest {N}",
    f"prime_winding factor {N}",
    f"prime_winding cycle {N}",
]

def run(N):
    args = [RUN] + CMDS(N)
    try:
        out = subprocess.run(args, capture_output=True, text=True, timeout=90).stdout
    except subprocess.TimeoutExpired as e:
        out = (e.stdout or b"").decode() if isinstance(e.stdout,bytes) else (e.stdout or "")
    return out

def grab(pat, s, g=1, cast=int):
    m = re.search(pat, s)
    if not m: return None
    try: return cast(m.group(g))
    except: return m.group(g)

def parse(N, out):
    d = {"N":N}
    d["period"]   = grab(r"period (\d+),", out)
    d["cuts"]     = grab(r"(\d+) winding-commit", out)
    d["crystal"]  = grab(r"crystal address\s*:\s*(\d+)", out)
    d["dialect"]  = grab(r"dialect register\s*:\s*([01]+)", out, 1, str)
    # squares / winding / bridge factor lines (N = a x b)
    facs = re.findall(rf"{N}\s*=\s*(\d+)\s*[x×]\s*(\d+)", out)
    d["factor_hit"] = facs[0] if facs else None
    d["oneshot"]  = grab(r"oneshot_prime_winder \d+[:\s]+([A-Za-z\-]+)", out, 1, str)
    d["dyn_depth"]= grab(r"depth\s*d?=?(\d+)", out)
    d["dyn_period"]=grab(r"[Pp]eriod\s*P?\(?d?\)?\s*=?\s*(\d+)", out)
    return d

def truth(N):
    f = factorint(N); ps = sorted(f)
    t = {"facs":ps}
    if len(ps)==2 and all(f[p]==1 for p in ps):
        p,q = ps
        lam = lcm(p-1,q-1)
        t.update(p=p,q=q,psum=p+q,pdif=q-p,lam=lam,
                 isqrt=isqrt(N), ceil_gap=(p+q)//2 - isqrt(N))
    return t

def main():
    Ns = [15,21,35,51,77,91,143,155,187,221,247,323,437,667,899,
          1517,3233,3599,8051,10403]
    rows=[]
    for N in Ns:
        out = run(N)
        open(f"factorizer_lattice/raw_{N}.txt","w").write(out)
        row = parse(N,out); row.update(truth(N))
        rows.append(row); print(row, flush=True)
    json.dump(rows, open("factorizer_lattice/lattice.json","w"), indent=1, default=str)
    print("WROTE factorizer_lattice/lattice.json")

main()
