from pathlib import Path
import subprocess,resource,json,time,hashlib,sys
label=sys.argv[1]
p=Path('measurements')/('idempotent_projector_'+label+'_20261021');p.mkdir(exist_ok=True)
source=Path(sys.argv[2]) if len(sys.argv)>2 else Path('measurements/idempotent_moment_100bit_20261021/source_0.txt')
n=source.read_text().strip()
binary=Path('target/x86_64-unknown-linux-gnu/release/examples/idempotent_moment')
def limits():
 resource.setrlimit(resource.RLIMIT_STACK,(64*1024*1024,64*1024*1024))
 resource.setrlimit(resource.RLIMIT_AS,(2*1024**3,2*1024**3))
t=time.monotonic()
with (p/'stdout').open('w') as out,(p/'stderr').open('w') as err:
 try:
  r=subprocess.run([str(binary),n,'--contract'],stdout=out,stderr=err,preexec_fn=limits,timeout=45);status=r.returncode
 except subprocess.TimeoutExpired:status='timeout'
receipt={'N':n,'source_file':str(source),'exit':status,'seconds':time.monotonic()-t,'memory_bound_bytes':2*1024**3,'stack_bytes':64*1024**2,'max_rss_kib':resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'sources':{f:hashlib.sha256(Path(f).read_bytes()).hexdigest() for f in ['src/idempotent_projector.rs','src/ququart_decision.rs','src/conventional_spectrum.rs']}}
(p/'receipt.json').write_text(json.dumps(receipt,indent=2));print(json.dumps(receipt))
