"""End-to-end paired-source preparation and read-off, without factor production."""
from pathlib import Path
import hashlib,json,os,secrets,subprocess,sys,time
from sympy import nextprime
OUT=Path(__file__).resolve().parent
REPO=OUT.parents[1]
ROOT=REPO.parent
KERNEL=REPO/'target/conventional_translation_20261020/x86_64-unknown-linux-gnu/release/g-momonados'

def run(label,args,expected=0,python=False,stdin=None):
    command=([sys.executable,str(ROOT/'conventional_translation.py')] if python else [str(KERNEL)])+args
    start=time.perf_counter()
    p=subprocess.run(command,input=stdin,capture_output=True,text=True,cwd=ROOT,
        env={**os.environ,'CONVENTIONAL_KERNEL':str(KERNEL)},timeout=120)
    (OUT/(label+'.stdout')).write_text(p.stdout)
    (OUT/(label+'.stderr')).write_text(p.stderr)
    assert p.returncode==expected,(label,p.returncode,p.stderr)
    return p,time.perf_counter()-start

def prime(bits):
    value=secrets.randbits(bits)|(3<<(bits-2))|1
    p=int(nextprime(value))
    return p if p.bit_length()==bits else prime(bits)

inputs=OUT/'inputs.json'
if not inputs.exists():
    rows=[]
    for bits in (200,512,896):
        p=prime(bits//2);q=prime(bits//2)
        while q==p:q=prime(bits//2)
        rows.append({'bits':bits,'name':str(p*q),'left':str(p),'right':str(q),
            'input_boundary':'Generated paired shape; no N-only factor production'})
    inputs.write_text(json.dumps(rows,indent=2)+'\n')
rows=json.loads(inputs.read_text());results=[]
for case in rows:
    bits=case['bits'];source=OUT/f'paired {bits}.json';receipt=OUT/f'both_{bits}.json'
    args=['conventional','multiply',case['left'],case['right'],'--write-source',str(source)]
    p,prepare_time=run(f'prepare_{bits}',args)
    prepared=json.loads(p.stdout)
    p,read_time=run(f'read_{bits}',['conventional','read','--source',str(source)])
    read=json.loads(p.stdout)['reading'];assert [read['left'],read['right']]==[case['left'],case['right']]
    assert read['factor_producer_calls']==0
    args=['both','--source',str(source),'--rank',str(2**256+13),'--value',str(2*int(case['name'])+17)]
    p,native_time=run(f'both_{bits}',['conventional',*args,'--receipt',str(receipt)])
    native=json.loads(p.stdout)
    assert native['source']==prepared['source']
    for report in native['reports']:
        assert len(report['slots'])==12
        assert report['native_operations_hold']
        self_model=report['slots'][0]['evidence']
        assert self_model['native_holographic'] and self_model['program_self_return']
        assert self_model['executed_trie_steps']>0
        assert [s['mark'] for s in report['slots']]==list('⊢⊣≻≺⋈⊤∈∋⊙⊥⊞⊡')
        assert [s['slot'] for s in report['slots'] if s['holds'] is None]==[1]
        assert all(s['holds'] is True for s in report['slots'] if s['holds'] is not None)
        assert report['self_output']==prepared['source']
        trees=report['arithmetic_self_reading']
        assert trees['closed'] and not trees['structures_identified']
        assert trees['binary_return']==trees['multiplicative_return']==case['name']
        assert trees['binary_popcount']==int(case['name']).bit_count()
        assert trees['retained_multiplicative_leaves']==2
        assert trees['prime_factor_count_omega'] is None
        e=report['slots'][8]['evidence']
        assert e['special_frobenius_return'] and e['spectral_idempotents'] and e['Y_fixed']
        assert e['Y_calls']>0 and e['Y_contractions']>0
        assert e['fused']=='17'
    p,python_time=run(f'python_{bits}',args,python=True)
    assert json.loads(p.stdout)==native
    run(f'require_all_{bits}',[*args,'--require-all'],expected=2,python=True)
    results.append({'bits':bits,'prepare_seconds':prepare_time,'read_process_seconds':read_time,
        'native_process_seconds':native_time,'python_process_seconds':python_time,'paired_projection_equal':True,
        'all_predicates_hold':False,'unresolved_specification_slots':[1]})
first=rows[0];source=OUT/'paired 200.json'
legacy=json.loads(source.read_text());legacy['schema']='paired-numeral/v1'
del legacy['binary_tree'];del legacy['multiplicative_tree']
legacy_path=OUT/'paired_v1.json';legacy_path.write_text(json.dumps(legacy))
p,_=run('legacy_source',['conventional','read','--source',str(legacy_path)])
assert json.loads(p.stdout)['source']['schema']=='paired-numeral/v2'
tree_tamper=json.loads(source.read_text());tree_tamper['binary_tree']['positions']=[]
tree_path=OUT/'tampered_tree.json';tree_path.write_text(json.dumps(tree_tamper))
run('tampered_tree_rejected',['conventional','read','--source',str(tree_path)],expected=1)
for n in (0,1,2**200):
    p,_=run(f'boundary_{n.bit_length()}', ['conventional','boundary',str(n)])
    boundary=json.loads(p.stdout);trees=boundary['return']
    assert trees['closed'] and trees['binary_popcount']==n.bit_count()
    assert boundary['zero_is_attractor_for_positive_inputs']==(n==0)
    assert trees['positive_prime_decomposition_domain']==(n!=0)
run('name_only_rejected',['conventional','dnn',first['name']],expected=1)
run('wrong_product_rejected',['conventional','dnn',str(int(first['name'])+1),'--legs',first['left'],first['right']],expected=1)
run('noncoprime_rejected',['conventional','prepare','--legs',first['left'],first['left']],expected=1)
tampered=json.loads(source.read_text());tampered['shape']['left_word']+='⊙'
bad=OUT/'tampered.json';bad.write_text(json.dumps(tampered,ensure_ascii=False))
run('tampered_word_rejected',['conventional','read','--source',str(bad)],expected=1)
# REPL token boundaries use a path without spaces.
repl_source=OUT/'paired_repl.json';repl_source.write_bytes(source.read_bytes())
p,repl_time=run('repl',[],stdin=f'conventional read --source {repl_source}\nexit\n')
assert 'stored-shape projection' in p.stdout and first['left'] in p.stdout
bindings={str(path.relative_to(ROOT)):hashlib.sha256(path.read_bytes()).hexdigest() for path in [
    ROOT/'conventional_translation.py',REPO/'src/conventional_translation.rs',REPO/'src/main.rs',REPO/'src/repl.rs',KERNEL,
    ROOT/'dualnn.txt',ROOT/'srfp.txt']}
(OUT/'result.json').write_text(json.dumps({'cases':results,'repl_seconds':repl_time,'sources':bindings,
    'status':'native paired-source read-off implemented; full predicate conjunction unresolved at slot 1'},indent=2)+'\n')
print(json.dumps(results,indent=2))
