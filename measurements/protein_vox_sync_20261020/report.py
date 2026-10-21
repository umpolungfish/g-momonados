"""Render the verified shared compiler's current architecture and evidence."""
from pathlib import Path
import json,hashlib
OUT=Path(__file__).resolve().parent;ROOT=OUT.parents[2]
r=json.loads((OUT/'result.json').read_text());assert r['status']=='passed' and len(r['installed'])==7
text='''# Shared Vox protein compiler, 2026-10-20

Standalone Vox, G-mOMonadOS direct argv and the hosted REPL now call [`Vox/src/protein_compile.rs`](../Vox/src/protein_compile.rs). G-mOMonadOS already resolves its runtime and build `vox_core` dependencies to `../Vox`; the older `vendor/vox` snapshot is outside that dependency path. Its duplicated REPL compiler has been removed.

The shared operation parses the selected input and code table, prepares all residue/form self-marks in `ProteinGlut`, computes secondary/contact observations and B4-aligned geometry, and returns the complete carrier with its source bindings. PDB export and carrier export share that same state. Direct G-mOMonadOS argv retains the existing Lean admission, preserves path arguments and propagates the compiler failure status. Its REPL invokes the same operation and remains interactive.

Both entry points accept `--fasta`, `--input auto|rna|dna|protein`, `--code standard|mitochondrial`, `--pdb`, `--carrier` and `--coords`. The `protein-cli` feature supplies hosted file and console I/O. Standalone defaults enable it; G-mOMonadOS enables it through `hosted`. `cargo check --lib --no-default-features` passed, preserving use of the core library without `std`.

## Executed checks

- Thirty retained UniProt proteins, 97–1464 residues, compiled in both directions through both executables. Their full carrier JSON and exact PDB hashes matched across executables and against the prior campaign's saved carriers.
- All twenty amino acids and twenty-two source forms passed in both directions, with every native transition table retained.
- Ten controls per executable checked help, invalid code/input/flags, missing values, failed writes, explicit protein ambiguity, mitochondrial translation, coordinate output and paths with spaces.
- An actual hosted REPL invocation produced the same all20 carrier as direct argv.
- Both release builds passed. Seven installed executable paths were atomically updated and exercised with an all20 carrier comparison; earlier executables remain beside them as `.before_protein_vox_sync_20261020` backups.

This establishes compiler/state/coordinate agreement. Native-fold accuracy remains unmeasured; the earlier geometry conflicts retain their original observations.

## Installed executables

'''
for i in r['installed']:text+=f"- `{i['path']}`\n"
text+='\n## Reproduction and bindings\n\n'
text+='Receipts, source and executable hashes, full PDB/carrier outputs, build logs and CLI/REPL logs: [`G-mOMonadOS/measurements/protein_vox_sync_20261020/`](../G-mOMonadOS/measurements/protein_vox_sync_20261020/). `verify.py` checks candidate binaries; `install.py` installs only a passing receipt. The original sequence provenance is retained in [`red-hot_rebis/measurements/global_protein_coverage_20261020/`](../red-hot_rebis/measurements/global_protein_coverage_20261020/). Its RNA inputs are canonical reverse translations, not native transcripts.\n'
p=ROOT/'ig-docs/shared_vox_protein_compiler_20261020.md';p.write_text(text)
p=ROOT/'ig-docs/membrane_reference_sources.json';d=json.loads(p.read_text());d['shared_vox_protein_compiler_20261020']={'date':'2026-10-20','implementation':'Vox/src/protein_compile.rs','entry_points':['Vox/src/main.rs','G-mOMonadOS/src/main.rs','G-mOMonadOS/src/repl.rs'],'review':'Shared executing producer, native carrier, geometry and export; runtime/build Cargo dependency paths and hosted feature','receipt':'G-mOMonadOS/measurements/protein_vox_sync_20261020/result.json','receipt_sha256':hashlib.sha256((OUT/'result.json').read_bytes()).hexdigest(),'report':'ig-docs/shared_vox_protein_compiler_20261020.md'};p.write_text(json.dumps(d,ensure_ascii=False,indent=2)+'\n')
print('Saved shared compiler architecture and verification report.')
