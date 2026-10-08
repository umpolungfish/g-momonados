"""Compare GPU ECM extraction latency with the CPU membrane on identical words."""
from pathlib import Path
import subprocess, time, json, re, statistics, sys, os, hashlib
ROOT=Path(__file__).resolve().parents[2]
OUT=Path(__file__).resolve().parent
CURVES=int(sys.argv[1]) if len(sys.argv)>1 else 1
results=[]
for bits in (256,512,1048):
    word=(ROOT/f"measurements/membrane_routes_128_20261007/scaling/ecm_witness/{bits}/source.imasm").read_text().strip()
    cpu=[]
    for i in range(5):
        start=time.perf_counter()
        p=subprocess.run([str(ROOT/"membranes/target/release/membrane_ecm_extract"),word,"100","100",str(CURVES),"6"],capture_output=True,text=True,check=True)
        cpu.append(time.perf_counter()-start)
        assert "product_closes=true" in p.stdout
        (OUT/f"cpu_{bits}_curves{CURVES}_{i}.log").write_text(p.stdout)
    command=f"gpu_ecm {word} 100 100 {CURVES} 6 0"
    commands=[f"gpu_ecm check {word} 0"]+[command]*7+["quit"]
    start=time.perf_counter()
    p=subprocess.run([str(ROOT/"target"/os.environ.get("ECM_PROFILE","release")/"g-momonados")],input="\n".join(commands)+"\n",capture_output=True,text=True,cwd=ROOT,check=True)
    wall=time.perf_counter()-start
    (OUT/f"gpu_optimized_{bits}_curves{CURVES}.log").write_text(p.stdout+p.stderr)
    gpu=[float(x) for x in re.findall(r"elapsed_seconds=([0-9.]+)",p.stdout)]
    assert len(gpu)==7 and p.stdout.count("product_closes=true")==7
    assert "closes=true" in p.stdout
    result={"gpu_source_sha256":hashlib.sha256((ROOT/"src/gpu_ecm_word.cu").read_bytes()).hexdigest(),"gpu_profile":os.environ.get("ECM_PROFILE","release"),"bits":bits,"B1":100,"B2":100,"sigma":6,"curves":CURVES,"cpu_seconds":cpu,"cpu_median":statistics.median(cpu),"gpu_seconds":gpu,"gpu_warm_median":statistics.median(gpu[2:]),"gpu_process_wall":wall,"gpu_warm_speedup":statistics.median(cpu)/statistics.median(gpu[2:])}
    results.append(result)
    (OUT/f"latency_curves{CURVES}.json").write_text(json.dumps(results,indent=2)+"\n")
    print(json.dumps(result),flush=True)
