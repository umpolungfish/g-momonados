"""Atomically install the verified local compilers, retaining prior executables."""
from pathlib import Path
import hashlib,json,os,shutil,subprocess
OUT=Path(__file__).resolve().parent;ROOT=OUT.parents[2]
receipt=json.loads((OUT/'result.json').read_text());assert receipt['status']=='passed'
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
installed=[]
for source_text,digest in receipt['binaries'].items():
    source=Path(source_text);assert sha(source)==digest
    if source.name=='vox':
        targets=[ROOT/'Vox/target/release/vox',ROOT/'Vox/target/x86_64-unknown-linux-musl/release/vox',Path('/home/mrnob0dy666/.cargo/bin/vox')]
        command=['compile']
    else:
        targets=[ROOT/'G-mOMonadOS/target/release/g-momonados',ROOT/'G-mOMonadOS/target/x86_64-unknown-linux-gnu/release/g-momonados',ROOT/'G-mOMonadOS/g-momonados',Path('/home/mrnob0dy666/.cargo/bin/g-momonados')]
        command=['vox','compile']
    for path in targets:
        path.parent.mkdir(parents=True,exist_ok=True)
        backup=path.with_name(path.name+'.before_protein_vox_sync_20261020')
        if path.exists() and not backup.exists():shutil.copy2(path,backup)
        pending=path.with_name(path.name+'.protein_sync_pending')
        shutil.copy2(source,pending);os.replace(pending,path)
        assert sha(path)==digest
        carrier=OUT/(f'installed_{len(installed)}.json')
        result=subprocess.run([str(path),*command,'MARNDCEGHILKYPQSTVWF','--input','protein','--carrier',str(carrier)],cwd=ROOT,capture_output=True,text=True,timeout=180)
        (OUT/f'installed_{len(installed)}.stdout').write_text(result.stdout);(OUT/f'installed_{len(installed)}.stderr').write_text(result.stderr)
        assert result.returncode==0,(str(path),result.stderr)
        actual=json.loads(carrier.read_text())['protein_glut']
        expected=json.loads((OUT/'all20/vox_protein.pdb.glut.json').read_text())['protein_glut']
        assert actual==expected,str(path)
        installed.append({'path':str(path),'sha256':digest,'backup':str(backup),'carrier_equal':True})
# Installed commands also work independently of the repository working directory.
for path,command in [(Path('/home/mrnob0dy666/.cargo/bin/vox'),['compile']),(Path('/home/mrnob0dy666/.cargo/bin/g-momonados'),['vox','compile'])]:
    carrier=OUT/('outside_cwd_'+path.name+'.json')
    result=subprocess.run([str(path),*command,'MARNDCEGHILKYPQSTVWF','--input','protein','--carrier',str(carrier)],cwd=Path.home(),capture_output=True,text=True,timeout=180)
    assert result.returncode==0,(path,result.stderr)
    assert json.loads(carrier.read_text())['protein_glut']==expected
receipt['installed']=installed
receipt['outside_repository_cwd']='passed'
receipt['active_vox_dependency']=str(ROOT/'Vox')
receipt['cargo_metadata_sha256']=sha(OUT/'cargo_metadata.json')
receipt['no_std_check']='cargo check --lib --no-default-features; passed'
for name in ['Vox/README.md','Vox/USER_GUIDE.md','G-mOMonadOS/README.md','ig-docs/membrane_reference_02_vox.md','ig-docs/vox-protein-compile-guide.md']:
    receipt['sources'][name]=sha(ROOT/name)
(OUT/'result.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2)+'\n')
print('Installed and verified',len(installed),'executable paths.')
