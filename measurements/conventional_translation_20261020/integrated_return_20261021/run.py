from pathlib import Path
import hashlib,json,resource,secrets,subprocess,time

BASE=Path(__file__).resolve().parent
PROJECT=BASE.parents[2]
TARGET=PROJECT/'target/conventional_translation_20261020/x86_64-unknown-linux-gnu/release'
SOURCE=BASE/'fresh_balanced_200.txt'

def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def limits():
    resource.setrlimit(resource.RLIMIT_AS,(8*1024**3,8*1024**3))
    resource.setrlimit(resource.RLIMIT_CORE,(0,0))
def run(name,command,bound=300):
    start=time.monotonic()
    with (BASE/(name+'.stdout')).open('w') as out,(BASE/(name+'.stderr')).open('w') as err:
        result=subprocess.run(['timeout','-s','INT','-k','5s',str(bound)+'s',*map(str,command)],stdout=out,stderr=err,preexec_fn=limits)
    record={'name':name,'command':list(map(str,command)),'returncode':result.returncode,
        'seconds':time.monotonic()-start,'external_bound_seconds':bound,'address_space_GiB':8}
    (BASE/(name+'_run.json')).write_text(json.dumps(record,indent=2)+'\n')
    print(json.dumps(record),flush=True)
    return result.returncode

if not SOURCE.exists():
    from sympy import nextprime
    while True:
        p=int(nextprime(secrets.randbits(100)|(1<<99)))
        q=int(nextprime(secrets.randbits(100)|(1<<99)))
        if p!=q and p.bit_length()==q.bit_length()==100 and (p*q).bit_length()==200:break
    SOURCE.write_text(str(p*q)+'\n')
    reference=BASE/'generator_reference';reference.mkdir(exist_ok=True)
    (reference/'legs.json').write_text(json.dumps({'p':str(p),'q':str(q),'purpose':'generator reference; never read by native instruments'})+'\n')

binding={'source_sha256':digest(SOURCE),'binary_sha256':digest(TARGET/'g-momonados'),
    'sources':{name:digest(PROJECT/'src'/name) for name in ['conventional_spectrum.rs','ququart_decision.rs','reentry_wells.rs','conventional_protocol.rs']}}
(BASE/'single_reduction_binding.json').write_text(json.dumps(binding,indent=2)+'\n')
for name,source in [('integer',BASE/'integer_control.rs'),('protocol',BASE.parent/'protocol_probe.rs'),('prefix',BASE.parent/'affine_rows/prefix_probe.rs')]:
    executable=BASE/(name+'_control')
    command=['rustc','--edition=2021','-C','opt-level=2','-C','panic=abort','--target','x86_64-unknown-linux-gnu',
        '-L','dependency='+str(TARGET/'deps'),'--extern','g_momonados='+str(TARGET/'libg_momonados.rlib')]
    if name=='integer':command+=['--extern','num_bigint='+str(TARGET/'deps/libnum_bigint-54692d642badf259.rlib')]
    subprocess.run([*command,str(source),'-o',str(executable)],check=True)
    if run(name,[executable,SOURCE]):raise SystemExit(1)

for name,source in [('fresh200',SOURCE),('exact896',BASE.parent/'point_projection/user_896.txt')]:
    run(name,[TARGET/'g-momonados','conventional','prepare',source.read_text().strip(),'both'])
