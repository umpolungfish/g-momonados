"""Integrated dispatch/rejection and bounded full-family checks. N-only inputs."""
from pathlib import Path
import hashlib,json,resource,subprocess,time
BASE=Path(__file__).resolve().parent
ROOT=BASE.parents[2]
OUT=BASE/'protocol_execution'
KERNEL=ROOT/'G-mOMonadOS/target/conventional_translation_20261020/x86_64-unknown-linux-gnu/release/g-momonados'
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def limits():resource.setrlimit(resource.RLIMIT_AS,(8*1024**3,8*1024**3))
checks=[]
for name,artifact in [('old_schema',{'schema':'conventional-prepared-point/v1','input_boundary':'N-only'}),('missing_protocol',{'schema':'conventional-prepared-point/v2','input_boundary':'N-only','preparation_complete':True,'status':'arithmetic-closed'}),('missing_trace',{'schema':'conventional-prepared-point/v2','input_boundary':'N-only','protocol_kind':'srfp','preparation_complete':True,'status':'arithmetic-closed'})]:
 p=subprocess.run([str(KERNEL),'conventional','read','-'],input=json.dumps(artifact),capture_output=True,text=True,timeout=10)
 assert p.returncode!=0,(name,p.stdout)
 checks.append({'name':name,'rejected':True,'error':p.stderr.strip() or p.stdout.strip()})
control=json.loads((OUT/'control_200.json').read_text())
assert control['source_bits']>=200
for c in control['controls']:
 assert c['native_residual']==c['expected_residual']=='2'
 assert c['postselection_rejected'] and c['shared_bank_roundtrip'] and c['parity_reversal_involution'] and c['escaped_bank_rejected']
 trace=c['execution'][0]
 assert trace['native_flow_agreement'] and trace['fixation_executed'] and len(trace['nodes'])==12
 assert [n['node'] for n in trace['nodes']]==list(range(12))
checks.append({'name':'all_native_nodes_and_numerical_banks','passed':True,'receipt_sha256':sha(OUT/'control_200.json')})
runs=[]
for label,path,kind in [('fresh_200',OUT/'fresh_balanced_200.txt','srfp'),('exact_896',BASE/'point_projection/user_896.txt','dnn')]:
 n=path.read_text().strip();assert int(n).bit_length()>=200
 command=['timeout','-s','INT','-k','5s','20s',str(KERNEL),'conventional','prepare',n,kind]
 started=time.monotonic()
 with (OUT/(label+'.stdout')).open('w') as stdout,(OUT/(label+'.stderr')).open('w') as stderr:
  p=subprocess.run(command,stdout=stdout,stderr=stderr,preexec_fn=limits)
 elapsed=time.monotonic()-started
 log=(OUT/(label+'.stderr')).read_text()
 assert f'protocol={kind} node=0 opcode=VINIT' in log
 assert f'protocol={kind} node=1 opcode=IMSCRIB' in log
 assert f'protocol={kind} node=2 opcode=AFWD' in log
 runs.append({'name':label,'N':n,'bits':int(n).bit_length(),'protocol':kind,'returncode':p.returncode,'elapsed_seconds':elapsed,'external_bound_seconds':20,'last_observation':log.splitlines()[-1],'stdout_bytes':(OUT/(label+'.stdout')).stat().st_size,'scope':'Actual complete-family N-only preparation; bounded observation, not a completed factor receipt'})
receipt={'schema':'conventional-protocol-integration/v1','binary_sha256':sha(KERNEL),'control_executable_sha256':sha(BASE/'protocol_probe'),'checks':checks,'runs':runs,'sources':{str(path.relative_to(ROOT)):sha(path) for path in [ROOT/'G-mOMonadOS/src/conventional_spectrum.rs',ROOT/'G-mOMonadOS/src/ququart_decision.rs',ROOT/'G-mOMonadOS/src/conventional_translation.rs',ROOT/'conventional_translation.py']}}
(OUT/'result.json').write_text(json.dumps(receipt,indent=2)+'\n')
print(json.dumps(receipt,indent=2))
