"""N-only projection diagnostic with resource boundaries outside production."""
from pathlib import Path
import hashlib,json,resource,subprocess,time,signal
ROOT=Path(__file__).resolve().parents[3]
BASE=Path(__file__).resolve().parent
OUT=BASE/'uncapped_projection'
OUT.mkdir(exist_ok=True)
KERNEL=ROOT/'G-mOMonadOS/target/conventional_translation_20261020/x86_64-unknown-linux-gnu/release/g-momonados'
def limits():
    resource.setrlimit(resource.RLIMIT_AS,(8*1024**3,8*1024**3))
    resource.setrlimit(resource.RLIMIT_STACK,(64*1024**2,64*1024**2))
rows=[]
for label in ['user_896','fresh_control_200']:
    n=(BASE/'point_projection'/(label+'.txt')).read_text().strip()
    start=time.perf_counter()
    command=[str(KERNEL),'conventional','both',n]
    row={'N':n,'bits':int(n).bit_length(),'input_boundary':'N-only','observation_seconds':600,'address_space_limit_bytes':8*1024**3,'binary_sha256':hashlib.sha256(KERNEL.read_bytes()).hexdigest()}
    with (OUT/(label+'.stdout')).open('w') as stdout,(OUT/(label+'.stderr')).open('w') as stderr:
        p=subprocess.Popen(command,stdout=stdout,stderr=stderr,cwd=ROOT,preexec_fn=limits)
        row['pid']=p.pid
        (OUT/'launch.json').write_text(json.dumps(row,indent=2)+'\n')
        try:
            p.wait(timeout=600)
            row['observation_timeout']=False
        except subprocess.TimeoutExpired:
            row['observation_timeout']=True
            p.send_signal(signal.SIGTERM);p.wait()
    row.update(returncode=p.returncode,process_seconds=time.perf_counter()-start)
    if p.returncode==0:
        result=json.loads((OUT/(label+'.stdout')).read_text())
        row.update({k:result[k] for k in ['status','factors','product_verified','point_projection','Y']})
        if result['factors']:
            a,b=map(int,result['factors'])
            assert 1<a<int(n) and a*b==int(n)
            row['independent_exact_product']=True
    else:
        row['status']='observation-boundary' if row['observation_timeout'] else 'execution-terminated'
    row['stderr_tail']=(OUT/(label+'.stderr')).read_text()[-2400:]
    rows.append(row)
    (OUT/'result.json').write_text(json.dumps({'cases':rows},indent=2)+'\n')
    print(json.dumps(row,indent=2),flush=True)
