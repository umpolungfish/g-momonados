"""N-only boundary and actual held-state execution. Never read reference legs."""
from pathlib import Path
import hashlib,json,os,subprocess,sys,time
ROOT=Path(__file__).resolve().parents[3]
OUT=Path(__file__).resolve().parent/'n_only'
OUT.mkdir(exist_ok=True)
KERNEL=ROOT/'G-mOMonadOS/target/conventional_translation_20261020/x86_64-unknown-linux-gnu/release/g-momonados'
USER_N='412023436986659543855531365332575948179811699844327982845455626433876445565248426198098870423161841879261420247188869492560931776375033421130982397485150944909106910269861031862704114880866970564902903653658867433731720813104105190864254793282601391257624033946373269391'
# These files are N-only, separately written from historical paired receipts.
values=[p.read_text().strip() for p in sorted((OUT/'inputs').glob('*.txt'))]
assert values and USER_N in values
rows=[]
for i,n in enumerate(values):
    assert n.isdecimal() and int(n).bit_length()>=200
    command=[str(KERNEL),'conventional','both',n]
    start=time.perf_counter();p=subprocess.run(command,capture_output=True,text=True,timeout=300)
    elapsed=time.perf_counter()-start
    assert p.returncode==0,p.stderr
    result=json.loads(p.stdout)
    assert result['input_boundary']=='N-only' and result['source']=={'schema':'conventional-n-only/v1','N':n}
    assert result['binary_tree']['evaluation']==n
    forms=result['numeral_forms']
    assert forms['native_evaluation']==forms['hex_evaluation']==n and forms['retained_through_Y']
    if n==USER_N:
        observed=(OUT/'tfactor_read_user.stdout').read_text()
        for label,key in [('native word','native_word'),('hex-digit word','hex_digit_word')]:
            expected=next(line.split(':',1)[1].strip() for line in observed.splitlines() if line.strip().startswith(label))
            assert forms[key]==expected,(label,'tfactor source form differs')
    assert result['binary_tree']['positions']==[k for k in range(int(n).bit_length()) if (int(n)>>k)&1]
    assert result['quotient']['modulus']==n and result['quotient']['shared_point_symbol']
    assert result['quotient']['fused_projection']=={'constant':'1','E':'0'}
    assert result['Y']['operator_calls']==2 and result['Y']['contractions']>0
    assert result['factors'] is None and not result['product_verified']
    assert all(r['pairing_normal_form_stable'] and not r['all_predicates_hold'] for r in result['reports'])
    (OUT/f'{i}.json').write_text(json.dumps(result,indent=2)+'\n')
    args=[sys.executable,str(ROOT/'conventional_translation.py'),'both',n]
    q=subprocess.run(args,capture_output=True,text=True,env={**os.environ,'CONVENTIONAL_KERNEL':str(KERNEL)},timeout=300)
    assert q.returncode==0,q.stderr
    other=json.loads(q.stdout)
    for x in [result,other]:x.pop('trigger_to_held_return_seconds')
    assert other==result
    # Existing operands cannot enter any production command or its file path.
    for extra in [['--legs','2','3'],['--source','paired.json']]:
        q=subprocess.run(command+extra,capture_output=True,text=True,timeout=300)
        assert q.returncode!=0
    rows.append({'N':n,'bits':int(n).bit_length(),'process_seconds':elapsed,
        'status':'held','factors':None,'product_verified':False})
repl=subprocess.run([str(KERNEL)],input=f'conventional both {USER_N}\nexit\n',capture_output=True,text=True,timeout=300)
assert repl.returncode==0 and 'conventional-n-only-return/v1' in repl.stdout
(OUT/'repl.stdout').write_text(repl.stdout)
paths=[KERNEL,ROOT/'conventional_translation.py',ROOT/'G-mOMonadOS/src/conventional_spectrum.rs',ROOT/'G-mOMonadOS/src/conventional_translation.rs',ROOT/'G-mOMonadOS/src/numeral_encodings.rs',ROOT/'G-mOMonadOS/src/prime_winding.rs']
(OUT/'result.json').write_text(json.dumps({'cases':rows,'sources':{str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest() for p in paths},
    'status':'N-only construction executes; nontrivial residue evaluation remains unresolved'},indent=2)+'\n')
print(json.dumps(rows,indent=2))
