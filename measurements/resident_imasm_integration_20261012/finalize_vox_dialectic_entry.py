"""Retain distinct host-preparation and target-runtime bindings for this build."""
import copy, hashlib, json, re, shutil, subprocess
from pathlib import Path
here=Path(__file__).resolve().parent
repo=here.parents[1]
tag="vox_dialectic_entry_locked_512"
def digest(p): return hashlib.sha256(p.read_bytes()).hexdigest()
receipt=here/f"{tag}.static-build.receipt.json"
r=json.loads(receipt.read_text())
assert r.get("exit_code")==0 and r["status"]=="complete", "Cargo must finish first"
old=r["bindings"]["implementation_sha256"]
changed=[p for p,h in old.items() if digest(repo/p)!=h]
assert changed==["src/resident_glut.rs"], changed
before=(here/f"{tag}.source/src/resident_glut.rs").read_text()
after=(repo/changed[0]).read_text()
assert before.replace('        let divisor_word:String = split.divisor.iter().collect();\n        let divisor = WordTape::from_canonical_word(&divisor_word)', '        let divisor = WordTape::from_parity_cells(&split.divisor)')==after
images=[p for p in (repo/"target/x86_64-unknown-linux-musl/release/build").glob("g-momonados-*/out/resident_prepared.rs") if p.stat().st_mtime>=r["started_epoch"]]
assert len(images)==1
image=images[0]
binary=repo/"target/x86_64-unknown-linux-musl/release/examples/resident_prepared"
# Cargo cannot compile the target crate until the host preparation finishes.
assert (repo/changed[0]).stat().st_mtime < image.stat().st_mtime < binary.stat().st_mtime
original=here/f"{tag}.preparation-build.receipt.json"
assert not original.exists()
shutil.copy2(receipt,original)
r["preparation_bindings"]=copy.deepcopy(r["bindings"])
r["runtime_source_correction"]={"path":changed[0],"change":"Decode SourceDivisor parity cells with from_parity_cells", "source_mtime":(repo/changed[0]).stat().st_mtime,"prepared_image_mtime":image.stat().st_mtime,"executable_mtime":binary.stat().st_mtime}
r["bindings"]["implementation_sha256"][changed[0]]=digest(repo/changed[0])
runtime=here/f"{tag}.runtime-source/src/resident_glut.rs"
runtime.parent.mkdir(parents=True,exist_ok=True)
shutil.copy2(repo/changed[0],runtime)
executable=here/f"{tag}.static"
shutil.copy2(binary,executable)
r["executable_sha256"]=digest(executable)
s=image.read_text()
r["preparation_nanoseconds"]=int(re.findall(r"const BAKED_PREPARATION_NS:u64=(\d+);",s)[0])
r["prepared_image_sha256"]=digest(image)
ancestry=json.loads(json.loads(re.findall(r'preparation_sic_json:("(?:\\.|[^"\\])*")',s)[0]))
for name in ["forward_audit_frame","reverse_audit_frame","prepared_operator_frame"]:
 frame=ancestry["circuit_braid"][name]
 assert frame["exact_banked_return"] and frame["information_arm_loss_detected"]
(here/f"{tag}.preparation_ancestry.json").write_text(json.dumps(ancestry,indent=2)+"\n")
inspection=subprocess.run(["readelf","-l","-d",str(executable)],capture_output=True,text=True,check=True).stdout
(here/f"{tag}.elf-inspection.txt").write_text(inspection)
r["static_elf_verified"]="INTERP" not in inspection and "(NEEDED)" not in inspection
assert r["static_elf_verified"]
receipt.write_text(json.dumps(r,indent=2)+"\n")
print(json.dumps({k:r[k] for k in ["status","cargo_wall_seconds","preparation_nanoseconds","executable_sha256","static_elf_verified"]}))
