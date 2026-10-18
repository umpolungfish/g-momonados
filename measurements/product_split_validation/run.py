import hashlib,json,re,subprocess,sys,time
from pathlib import Path
root=Path('measurements/product_split_validation');name=sys.argv[1];seconds=int(sys.argv[2]);tag=sys.argv[3]
word=(root/'inputs'/f'{name}.imasm').read_text().strip()
binary=Path('target/release/g-momonados')
binary_binding=hashlib.sha256(binary.read_bytes()).hexdigest()
source_bindings={f:hashlib.sha256(Path(f).read_bytes()).hexdigest() for f in ['src/product_split.rs','src/factor_membrane.rs','src/native_numeral.rs','src/word_tape.rs']}
start=time.monotonic()
p=subprocess.run([str(binary),'factor_membrane','separate',word,str(seconds)],capture_output=True,text=True,timeout=seconds+90)
elapsed=time.monotonic()-start
(root/f'{tag}.stdout').write_text(p.stdout);(root/f'{tag}.stderr').write_text(p.stderr)
report={'input_name':name,'input_sha256':hashlib.sha256((root/'inputs'/f'{name}.imasm').read_bytes()).hexdigest(),'binary_sha256':binary_binding,'budget_seconds':seconds,'driver_seconds':elapsed,'returncode':p.returncode,'closed':'CLOSED: joint product split' in p.stdout,'open':'OPEN: product split retained' in p.stdout,'source_boundary':'N-only canonical word; no reference factors or factor widths supplied','source_bindings':source_bindings}
match=re.search(r'^continuation=(.+)$',p.stdout,re.M)
if match:
 path=Path(match.group(1));state=json.loads(path.read_text());assert state['source_word']==word
 report.update(continuation=str(path),continuation_sha256=hashlib.sha256(path.read_bytes()).hexdigest(),nodes=state['nodes'],low_steps=state['low_steps'],high_steps=state['high_steps'],interval_rejections=state['interval_rejections'],retained_branches=len(state['frontier']),quotient_forced_bits=state.get('quotient_forced_bits',0))
 # Preserve each bounded return before the next re-entry overwrites the live state.
 (root/f'{tag}.state.json').write_text(json.dumps(state,indent=2)+'\n')
if report['closed']:
 fields=dict(re.findall(r'^(p|q)=([^\n]+)$',p.stdout,re.M));values={}
 for key,w in fields.items():
  output=subprocess.check_output([str(binary),'native_numeral','decode',w],text=True)
  # The decode report retains the explicit decimal reading.
  values[key]=output
 report['factor_decode_reports']=values
 for key in ['proper_divisor','zero_remainder','quotient_matches','exact_word_product','paired_boundary']:assert f'{key}=true' in p.stdout
(root/f'{tag}.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report),flush=True)
assert p.returncode==0 and (report['closed'] or report['open']),p.stdout+p.stderr
