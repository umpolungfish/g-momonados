"""Bounded N-only integrated point projection; never consumes reference factors."""
from pathlib import Path
import hashlib,json,resource,subprocess,time,signal
ROOT=Path(__file__).resolve().parents[3]
OUT=Path(__file__).resolve().parent/'point_projection'
OUT.mkdir(exist_ok=True)
KERNEL=ROOT/'G-mOMonadOS/target/conventional_translation_20261020/x86_64-unknown-linux-gnu/release/g-momonados'

def limits():
    resource.setrlimit(resource.RLIMIT_AS,(4*1024**3,4*1024**3))
    resource.setrlimit(resource.RLIMIT_STACK,(64*1024**2,64*1024**2))

rows=[]
for label in ['user_896','fresh_control_200']:
    n=(OUT/(label+'.txt')).read_text().strip()
    assert n.isdecimal() and int(n).bit_length()>=200
    command=[str(KERNEL),'conventional','both',n]
    start=time.perf_counter();timed_out=False
    with (OUT/(label+'.stdout')).open('w') as stdout,(OUT/(label+'.stderr')).open('w') as stderr:
        p=subprocess.Popen(command,stdout=stdout,stderr=stderr,cwd=ROOT,preexec_fn=limits)
        try:p.wait(timeout=330)
        except subprocess.TimeoutExpired:
            timed_out=True;p.send_signal(signal.SIGINT)
            try:p.wait(timeout=5)
            except subprocess.TimeoutExpired:p.kill();p.wait()
    row={'N':n,'bits':int(n).bit_length(),'input_boundary':'N-only','process_seconds':time.perf_counter()-start,
        'returncode':p.returncode,'observation_timeout':timed_out,'address_space_limit_bytes':4*1024**3}
    if p.returncode==0:
        returned=json.loads((OUT/(label+'.stdout')).read_text())
        assert returned['source']=={'schema':'conventional-n-only/v1','N':n}
        assert returned['numeral_forms']['native_evaluation']==returned['numeral_forms']['hex_evaluation']==n
        row.update({k:returned[k] for k in ['status','factors','product_verified','point_projection','Y']})
        if returned['factors']:
            a,b=map(int,returned['factors']);assert 1<a<int(n) and 1<b<int(n) and a*b==int(n)
            row['independent_exact_product']=True
        else:assert not returned['product_verified']
    else:
        row['status']='execution-terminated';row['stderr_tail']=(OUT/(label+'.stderr')).read_text()[-1800:]
    rows.append(row)
    (OUT/'result.json').write_text(json.dumps({'cases':rows,'binary_sha256':hashlib.sha256(KERNEL.read_bytes()).hexdigest()},indent=2)+'\n')
    print(json.dumps(row,indent=2),flush=True)
