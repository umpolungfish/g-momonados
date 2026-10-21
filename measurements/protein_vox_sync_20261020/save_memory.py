"""Save the shared compiler's verified source and executable bindings."""
from pathlib import Path
import json
OUT=Path(__file__).resolve().parent
r=json.loads((OUT/'result.json').read_text());assert r['status']=='passed' and len(r['installed'])==7
MEM=Path('/home/mrnob0dy666/.codex/memories/imsgct')
name='shared_vox_protein_compiler_20261020.md'
text='''# Shared Vox protein compiler, 2026-10-20

Lando requests that Vox implemented in Vox and G-mOMonadOS both be brought up to date.

G-mOMonadOS already links vox_core to ../Vox, but its REPL vox compile branch had duplicated the older Rebis compiler. Extracted the complete hosted compiler to Vox/src/protein_compile.rs. Standalone main, G-mOMonadOS direct argv and its REPL call the same run function. Hosted I/O is gated by protein-cli; default standalone enables it, G hosted enables vox_core/protein-cli, and cargo check --lib --no-default-features still passes. G's vendor/vox snapshot is not the active Cargo dependency.

Both interfaces support --fasta, --input auto|rna|dna|protein, --code, --pdb, --carrier and --coords. Full self-mark execution precedes folding; exact source-bound carriers, geometry evidence and SHA-bound PDB sidecars return together. Direct G argv preserves path arguments and compiler failure status, after existing Lean admission. The interactive REPL remains interactive on errors.

G-mOMonadOS/measurements/protein_vox_sync_20261020/ contains verify.py, result.json, build logs, no_std check, complete carriers, CLI controls and REPL receipts. Thirty retained UniProt chains in both directions plus all20/22forms produce identical carrier JSON and exact PDB hashes through both executable paths. Saved previous campaign carriers also match. Ten CLI controls per executable cover flags, code rejection, ambiguity, errors, coordinates and paths with spaces. Actual REPL export matches direct output. This verifies compilation/state/coordinate agreement; native fold accuracy remains unmeasured and existing clashes remain evidence.

Native candidate is Vox/target/protein_integration_20261020/x86_64-unknown-linux-musl/release/vox. Hosted candidate is G-mOMonadOS/target/protein_sync_20261020/x86_64-unknown-linux-gnu/release/g-momonados. Both installed with atomic replacement and .before_protein_vox_sync_20261020 backups. Seven paths verified: Vox target/release, target/musl/release, ~/.cargo/bin/vox; G target/release, target/gnu/release, repository-root g-momonados and ~/.cargo/bin/g-momonados. result.json binds hashes and sources.

README and Vox USER_GUIDE describe the shared entry. ig-docs/membrane_reference_02_vox.md is current; vox-protein-compile-guide.md labels its August source layout and quoted outputs historical beneath a current-interface note. Earlier receipts retain their original source/binary hashes.
'''
(MEM/name).write_text(text)
header='SHARED VOX PROTEIN COMPILER 2026-10-20: Standalone and G-mOMonadOS direct/REPL vox compile now execute Vox/src/protein_compile.rs. Both rebuilt; 30proteins in both directions plus all20/22forms, 20CLI controls and REPL pass exact carrier/PDB equality. All7 executable locations updated with backups. Sources/receipts: G-mOMonadOS/measurements/protein_vox_sync_20261020/. See [[shared_vox_protein_compiler_20261020]].\n\n'
p=MEM/'CURRENT.md';s=p.read_text()
if header not in s:p.write_text(header+s)
p=MEM/'MEMORY.md';s=p.read_text();entry='- [[shared_vox_protein_compiler_20261020]]: shared native/hosted compiler, 30chain equality, CLI/REPL controls and all installed executable bindings.\n'
if entry not in s:p.write_text(s.rstrip()+'\n\n'+entry)
print('Saved shared Vox compiler memory and index.')
