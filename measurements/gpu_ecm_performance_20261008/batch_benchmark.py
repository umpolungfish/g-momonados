"""Time complete identical stage-one curve batches, including process startup."""
from pathlib import Path
import subprocess,time,json,re,os,hashlib
ROOT=Path(__file__).resolve().parents[2]; OUT=Path(__file__).resolve().parent
word=(ROOT/"measurements/membrane_routes_128_20261007/scaling/256/source.imasm").read_text().strip()
report=[]
for curves in (32,256):
    start=time.perf_counter()
    cpu=subprocess.run([str(ROOT/"membranes/target/release/membrane_ecm_extract"),word,"100","100",str(curves),"6"],capture_output=True,text=True,check=True)
    cpu_time=time.perf_counter()-start
    (OUT/f"cpu_batch_{curves}.log").write_text(cpu.stdout)
    print(f"CPU {curves} curves {cpu_time:.6f} seconds",flush=True)
    start=time.perf_counter()
    gpu=subprocess.run([str(ROOT/"target"/os.environ.get("ECM_PROFILE","release")/"g-momonados")],input=f"gpu_ecm {word} 100 100 {curves} 6 0\nquit\n",capture_output=True,text=True,cwd=ROOT,check=True)
    gpu_wall=time.perf_counter()-start
    (OUT/f"gpu_batch_{curves}.log").write_text(gpu.stdout+gpu.stderr)
    assert "no factor" in cpu.stdout and "no factor" in gpu.stdout
    times=re.findall(r"elapsed_seconds=([0-9.]+)",gpu.stdout);assert len(times)==1
    result={"gpu_source_sha256":hashlib.sha256((ROOT/"src/gpu_ecm_word.cu").read_bytes()).hexdigest(),"gpu_profile":os.environ.get("ECM_PROFILE","release"),"bits":256,"B1":100,"B2":100,"first_sigma":6,"curves":curves,"cpu_process_seconds":cpu_time,"gpu_process_seconds":gpu_wall,"gpu_route_seconds":float(times[0]),"process_speedup":cpu_time/gpu_wall,"both_complete_without_factor":True}
    report.append(result);(OUT/"batch.json").write_text(json.dumps(report,indent=2)+"\n");print(json.dumps(result),flush=True)
