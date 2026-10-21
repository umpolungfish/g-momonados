"""Compare both executable compiler paths on the retained coverage matrix."""
from pathlib import Path
import hashlib,json,subprocess,time
OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[2]
DATA=ROOT/'red-hot_rebis/measurements/global_protein_coverage_20261020'
VOX=ROOT/'Vox/target/protein_integration_20261020/x86_64-unknown-linux-musl/release/vox'
GM=ROOT/'G-mOMonadOS/target/protein_sync_20261020/x86_64-unknown-linux-gnu/release/g-momonados'
BINARIES={'vox':[str(VOX),'compile'],'gm':[str(GM),'vox','compile']}
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def load(p):return json.loads(Path(p).read_text())
def run(kind,args,prefix,ok=True,stdin=None):
    started=time.monotonic()
    command=BINARIES[kind]+list(map(str,args))
    p=subprocess.run(command,cwd=ROOT,input=stdin,capture_output=True,text=True,timeout=180)
    Path(str(prefix)+'.stdout').write_text(p.stdout);Path(str(prefix)+'.stderr').write_text(p.stderr)
    assert (p.returncode==0)==ok,(command,p.returncode,p.stderr[-1500:])
    return {'command':command,'exit_code':p.returncode,'seconds':time.monotonic()-started}

def check_case(accession,args,length,forms=None):
    folder=OUT/accession;folder.mkdir(exist_ok=True)
    receipt={'accession':accession,'length':length,'directions':{}}
    for direction,options in args.items():
        carriers={};pdbs={};logs={}
        for kind in BINARIES:
            pdb=folder/f'{kind}_{direction}.pdb'
            logs[kind]=run(kind,options+['--pdb',pdb],folder/f'{kind}_{direction}')
            sidecar=load(str(pdb)+'.glut.json')
            assert sidecar['pdb_sha256']==sha(pdb)
            carrier=sidecar['protein_glut'];assert carrier['native_state']=='B'
            assert len(carrier['residues'])==length
            assert all(len(m['execution']['native_transition_table'])==16
                       for r in carrier['residues'] for m in r['self_marks'])
            if forms:assert sum(len(r['self_marks']) for r in carrier['residues'])==forms
            carriers[kind]=sidecar;pdbs[kind]=sha(pdb)
        assert carriers['vox']==carriers['gm'],(accession,direction,'carrier divergence')
        assert pdbs['vox']==pdbs['gm'],(accession,direction,'coordinate divergence')
        # Preserve the previous campaign's complete output as a regression reference.
        reference=DATA/accession/f'final_{direction}.pdb.glut.json'
        if reference.exists():assert carriers['vox']==load(reference),(accession,direction,'prior carrier divergence')
        receipt['directions'][direction]={'commands':logs,'carrier_equal':True,'pdb_sha256':pdbs['vox']}
    (folder/'result.json').write_text(json.dumps(receipt,indent=2)+'\n')
    print(accession,'passed',flush=True)
    return receipt

manifest=load(DATA/'manifest.json');cases=[]
for entry in manifest['entries']:
    a=entry['accession'];rna=(DATA/a/'canonical.rna').read_text().strip()
    cases.append(check_case(a,{'protein':['--fasta',DATA/entry['fasta_path'],'--input','protein'],
                              'rna':[rna,'--input','rna']},entry['length']))
all20='MARNDCEGHILKYPQSTVWF'
rna='AUGGCUCGUAAUGAUUGUGAAGGUCACAUUCUUAAAUAUCCUCAAUCUACUGUUUGGUUUUAA'
cases.append(check_case('all20',{'protein':[all20,'--input','protein'],'rna':[rna,'--input','rna']},20,22))
controls={}
for kind in BINARIES:
    base=OUT/('controls_'+kind);base.mkdir(exist_ok=True)
    controls[kind]=[]
    for name,args,ok in [
        ('help',['--help'],True),('invalid_code',[all20,'--code','invalid'],False),
        ('invalid_input',[all20,'--input','rna'],False),('unknown_flag',[all20,'--bogus'],False),
        ('missing_value',[all20,'--carrier'],False),
        ('bad_output',[all20,'--input','protein','--pdb',base/'missing/invalid.pdb'],False),
        ('ambiguous_protein',['ACGT','--input','protein','--carrier',base/'ambiguous.json'],True),
        ('mitochondrial',['AUGUGAUAA','--input','rna','--code','mitochondrial','--carrier',base/'mito.json'],True),
        ('coords',[all20,'--input','protein','--coords'],True),
        ('space_path',[all20,'--input','protein','--pdb',base/'path with spaces.pdb'],True)]:
        controls[kind].append({'name':name,**run(kind,args,base/name,ok)})
    assert len(load(base/'ambiguous.json')['protein_glut']['residues'])==4
    assert [r['aa'] for r in load(base/'mito.json')['protein_glut']['residues']]==['Met','Trp']
    assert 'Per-residue coordinates' in (base/'coords.stdout').read_text()
# Enter through the hosted REPL dispatch as well as direct argv.
repl_pdb=OUT/'repl.pdb'
p=subprocess.run([str(GM)],input=f'vox compile {all20} --input protein --pdb {repl_pdb}\nquit\n',
                 cwd=ROOT,capture_output=True,text=True,timeout=180)
(OUT/'repl.stdout').write_text(p.stdout);(OUT/'repl.stderr').write_text(p.stderr)
assert p.returncode==0,(p.returncode,p.stderr)
assert load(str(repl_pdb)+'.glut.json')==load(OUT/'all20/gm_protein.pdb.glut.json')
sources=['Vox/src/protein_compile.rs','Vox/src/protein_glut.rs','Vox/src/main.rs','Vox/src/lib.rs',
         'Vox/Cargo.toml','G-mOMonadOS/Cargo.toml','G-mOMonadOS/src/main.rs','G-mOMonadOS/src/repl.rs']
receipt={'status':'passed','coverage_cases':len(manifest['entries']),'all20_forms':22,'cases':cases,
         'controls':controls,'repl_carrier_equal':True,'binaries':{str(p):sha(p) for p in (VOX,GM)},
         'sources':{p:sha(ROOT/p) for p in sources},'manifest_sha256':sha(DATA/'manifest.json'),
         'scope':'Cross-executable compiler, native state and exported coordinate equality; native-fold accuracy remains unmeasured.'}
(OUT/'result.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2)+'\n')
print('Both executable paths and REPL passed',flush=True)
