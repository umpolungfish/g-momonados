from pathlib import Path
import subprocess,json,time,hashlib,resource
ROOT=Path(__file__).resolve().parents[3]
BASE=Path(__file__).resolve().parent
OUT=BASE/'flatten_profile';OUT.mkdir(exist_ok=True)
KERNEL=ROOT/'G-mOMonadOS/target/conventional_translation_20261020/x86_64-unknown-linux-gnu/release/g-momonados'
n=(BASE/'point_projection/fresh_control_200.txt').read_text().strip()
assert int(n).bit_length()>=200
command=['timeout','-s','INT','-k','5s','20s','perf','record','-F','99','--call-graph','dwarf,8192','-o',str(OUT/'before.perf.data'),'--',str(KERNEL),'conventional','prepare',n]
def limits():
 resource.setrlimit(resource.RLIMIT_AS,(8*1024**3,8*1024**3))
start=time.perf_counter()
with (OUT/'before.stdout').open('w') as out,(OUT/'before.stderr').open('w') as err:
 result=subprocess.run(command,stdout=out,stderr=err,preexec_fn=limits)
receipt={'scope':'20-second CPU profile of the actual N-only preparation, no factor receipt','N':n,'bits':int(n).bit_length(),'returncode':result.returncode,'elapsed_seconds':time.perf_counter()-start,'binary_sha256':hashlib.sha256(KERNEL.read_bytes()).hexdigest()}
(OUT/'before.json').write_text(json.dumps(receipt,indent=2)+'\n')
print(json.dumps(receipt,indent=2))
