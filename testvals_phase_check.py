"""Run every ../testvals.txt target through the silent nested phase family."""
from pathlib import Path
import subprocess
import time

ROOT=Path(__file__).resolve().parent
BIN=ROOT/"target"/"release"/"g-momonados"
VALUES=ROOT.parent/"testvals.txt"
LIMIT=1.0
rows=[]
for index,text in enumerate(VALUES.read_text().split(),1):
    n=int(text)
    width=(n.bit_length()+1)//2
    start=time.perf_counter()
    try:
        proc=subprocess.run([str(BIN),"--selector-relation",text,str(width),"1024","64"],
            cwd=ROOT,text=True,capture_output=True,timeout=LIMIT,check=True)
        elapsed=time.perf_counter()-start
        status="terminal" if "(verified)" in proc.stdout else "empty"
        output=proc.stdout.replace("\t"," ").replace("\n","\\n")
    except subprocess.TimeoutExpired as exc:
        elapsed=time.perf_counter()-start
        status="no_terminal_within_limit"
        output=((exc.stdout or b"").decode() if isinstance(exc.stdout,bytes) else (exc.stdout or ""))
        assert output=="",(index,output)
        output="-"
    rows.append((index,len(text),n.bit_length(),width,status,elapsed,output))
out=ROOT/"measurements"/"testvals_phase_relations.tsv"
out.write_text("index\tdigits\tproduct_bits\tfactor_width\tstatus\tseconds\tterminal_output\n"+
    "".join(f"{i}\t{d}\t{b}\t{w}\t{s}\t{t:.6f}\t{o}\n" for i,d,b,w,s,t,o in rows))
print(f"tested={len(rows)} terminal={sum(r[4]=='terminal' for r in rows)} silent_timeouts={sum(r[4]=='no_terminal_within_limit' for r in rows)} limit={LIMIT:.1f}s")
