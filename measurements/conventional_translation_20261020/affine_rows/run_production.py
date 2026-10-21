from pathlib import Path
import subprocess,json,hashlib,time,resource
BASE=Path(__file__).resolve().parent
PROJECT=BASE.parents[2];KERNEL=PROJECT/'target/conventional_translation_20261020/x86_64-unknown-linux-gnu/release/g-momonados'
receipt={'binary_sha256':hashlib.sha256(KERNEL.read_bytes()).hexdigest(),'runs':[]}
def limits():resource.setrlimit(resource.RLIMIT_AS,(8*1024**3,8*1024**3))
for name,path,kind in [('fresh_200',BASE/'fresh_balanced_200.txt','srfp'),('exact_896',BASE.parent/'point_projection/user_896.txt','dnn')]:
 n=path.read_text().strip();assert int(n).bit_length()>=200
 start=time.monotonic()
 with (BASE/(name+'.stdout')).open('w') as out,(BASE/(name+'.stderr')).open('w') as err:
  p=subprocess.run(['timeout','-s','INT','-k','5s','20s',str(KERNEL),'conventional','prepare',n,kind],stdout=out,stderr=err,preexec_fn=limits)
 lines=(BASE/(name+'.stderr')).read_text().splitlines()
 row={'name':name,'N':n,'bits':int(n).bit_length(),'protocol':kind,'returncode':p.returncode,'elapsed_seconds':time.monotonic()-start,'last_observation':lines[-1] if lines else None,'external_bound_seconds':20,'address_space_limit_GiB':8,'stdout_bytes':(BASE/(name+'.stdout')).stat().st_size}
 receipt['runs'].append(row);(BASE/'production.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(row),flush=True)
