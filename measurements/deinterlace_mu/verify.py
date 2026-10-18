import hashlib,json,re,subprocess
from pathlib import Path
binary=Path('target/release/g-momonados')
n='1443886668465785239555226711752594674554632143872728785072260425128493389058547185473075869'
word=subprocess.check_output([str(binary),'native_numeral','encode',n],text=True).strip()
p=subprocess.run([str(binary)],input='native_numeral deinterlace '+word+'\nquit\n',text=True,capture_output=True,timeout=30)
assert p.returncode==0,p.stderr
text=p.stdout+p.stderr
Path('measurements/deinterlace_mu/readout.log').write_text(text)
def field(label):
 return re.search(r'^'+re.escape(label)+r' = (.+)$',text,re.M).group(1).strip()
left,right=field('p'),field('q')
standalone=subprocess.check_output([str(binary),'native_numeral','mu',left,right],text=True)
Path('measurements/deinterlace_mu/standalone.log').write_text(standalone)
standalone_value=re.search(r'\(decode: (\d+)\)',standalone).group(1)
assert field('μ(p,q)')==standalone_value==n
assert field('encode(μ(p,q))')==word
assert 'μ(p,q)==decode(D): true' in text
assert 'syzygy preserves [parent; Γ; Λ; μ]: true' in text
assert field('plain multiply p*q')==str(int(left)*int(right))
assert 'p*q==decode(D): false' in text
paths=[binary,Path('target/x86_64-unknown-linux-gnu/release/g-momonados')]
bindings={str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}
assert len(set(bindings.values()))==1
result={'status':'passed','input':n,'lanes':[left,right],'mu_reconstructed_parent':standalone_value,'mu_word_equals_input':True,'plain_product':field('plain multiply p*q'),'plain_product_equals_parent':False,'binaries':bindings,'scope':'REPL deinterlace matches standalone existing mu on its returned lanes, including exact parent word; ordinary multiplication remains separate.'}
Path('measurements/deinterlace_mu/verification.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result))
