#!/usr/bin/env python3
"""Relocate the completed dialect campaign and archive historical executables."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import stat
import struct
import subprocess
import time
import zipfile

ROOT = Path(__file__).resolve().parents[1]
WORKSPACE = ROOT.parent
CAMPAIGN = ROOT / 'measurements/dialect_transport_20261017'
KEEP_EXECUTABLES = {'locked_512_r23', 'environment_kernel_full_tuple_20261018'}


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def is_executable_image(path):
    with path.open('rb') as stream:
        header = stream.read(18)
    return len(header) == 18 and header[:4] == b'\x7fELF' and struct.unpack_from('<H', header, 16)[0] in (2, 3)


def commit_repository(destination, message):
    if not (destination / '.git').exists():
        subprocess.run(['git', 'init', '-b', 'main', str(destination)], check=True, capture_output=True)
    subprocess.run(['git', '-C', str(destination), 'config', 'core.filemode', 'false'], check=True)
    for key in ('user.name', 'user.email'):
        configured = subprocess.run(['git', '-C', str(ROOT), 'config', key], capture_output=True, text=True)
        if configured.returncode == 0:
            subprocess.run(['git', '-C', str(destination), 'config', key, configured.stdout.strip()], check=True)
    (destination / 'commit.txt').write_text(message + '\n')
    subprocess.run(['git', '-C', str(destination), 'add', 'README.md', '.gitignore', 'commit.txt', 'restore_executable.py', 'documents', 'logs'], check=True)
    subprocess.run(['git', '-C', str(destination), 'commit', '-F', str(destination / 'commit.txt')], check=True, capture_output=True)
    print(subprocess.check_output(['git', '-C', str(destination), 'log', '-1', '--oneline'], text=True).strip(), flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('destination', type=Path)
    parser.add_argument('--stage-only', action='store_true', help='verify and commit the D: copies without changing any source paths')
    args = parser.parse_args()
    destination = args.destination.resolve()
    assert destination.parent == Path('/mnt/d'), 'use a repository directly on D:'
    if destination.exists():
        assert not (destination / '.git').exists() and (destination / '.gitignore').read_text() == '/artifacts/\n', 'destination is not our incomplete migration'
        for existing in destination.rglob('*'):
            if existing.is_dir() or existing == destination / '.gitignore':continue
            relative = existing.relative_to(destination)
            if relative == Path('artifacts/archives/dialect_executables_20261018.zip'):continue
            if relative.parts[:2] in (('logs','dialect_transport_20261017'),('artifacts','dialect_transport_20261017')):
                source = CAMPAIGN.joinpath(*relative.parts[2:])
                if source.is_file():assert digest(source) == digest(existing), 'incomplete destination contains differing data'
                else:print('Preserving staged record removed by concurrent source cleanup: ' + str(relative), flush=True)
            else:raise AssertionError('unexpected file in incomplete destination: ' + str(relative))
    assert CAMPAIGN.is_dir() and not CAMPAIGN.is_symlink()
    files = sorted(p for p in CAMPAIGN.rglob('*') if p.is_file())
    assert all(not p.is_symlink() for p in files), 'review existing links before relocation'
    originals = {p: (p.stat().st_size, p.stat().st_mtime_ns) for p in files}
    documents = sorted((WORKSPACE / 'ig-docs').glob('dialect_factor_transport_20261017.*'))
    assert documents and all(p.is_file() and not p.is_symlink() for p in documents)
    destination.mkdir(exist_ok=True)
    artifacts = destination / 'artifacts/dialect_transport_20261017'
    logs = destination / 'logs/dialect_transport_20261017'
    archives = destination / 'artifacts/archives'
    for folder in (artifacts, logs, archives, destination / 'documents'):
        folder.mkdir(parents=True, exist_ok=True)
    (destination / '.gitignore').write_text('/artifacts/\n')
    records = []
    objects = {}
    copied = {}
    last_update = time.monotonic()
    total_bytes = 0
    zip_path = archives / 'dialect_executables_20261018.zip'
    print('Copying progress records and archiving historical ELF executables on D:', flush=True)
    with zipfile.ZipFile(zip_path, 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=6, allowZip64=True) as archive:
        for index, source in enumerate(files):
            relative = source.relative_to(CAMPAIGN)
            size, _ = originals[source]
            sha = digest(source)
            record = {'path': str(relative), 'bytes': size, 'sha256': sha, 'mode': stat.S_IMODE(source.stat().st_mode)}
            if is_executable_image(source) and str(relative) not in KEEP_EXECUTABLES:
                entry = objects.get(sha)
                if entry is None:
                    entry = 'objects/' + sha + '.elf'
                    archive.write(source, entry)
                    objects[sha] = entry
                record.update(storage='executable-zip', entry=entry)
            else:
                # Keep human logs and small structured receipts in Git. Large
                # state datasets/build payloads stay on D: without a Git copy.
                small_record = size <= 1024 * 1024 and source.suffix in ('.json', '.log', '.stderr', '.txt')
                target = (logs if small_record else artifacts) / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                # Store identical completed artifacts once, preserving their
                # original logical paths with links inside the D: repository.
                canonical = copied.get(sha)
                if canonical is not None and size >= 1024 * 1024:
                    if target.exists() or target.is_symlink():
                        assert digest(target) == sha
                        target.unlink()
                    target.symlink_to(os.path.relpath(canonical, target.parent))
                    assert digest(target) == sha
                else:
                    if not target.exists():shutil.copyfile(source, target)
                    assert digest(target) == sha, 'copy checksum differs: ' + str(relative)
                    copied[sha] = target
                if small_record:
                    link = artifacts / relative
                    link.parent.mkdir(parents=True, exist_ok=True)
                    if not link.is_symlink():link.symlink_to(os.path.relpath(target, link.parent))
                    else:assert link.resolve() == target.resolve()
                record.update(storage='file', destination=str(target.relative_to(destination)))
            assert (source.stat().st_size, source.stat().st_mtime_ns) == originals[source], 'source changed during copy'
            records.append(record)
            total_bytes += size
            if time.monotonic() - last_update >= 25:
                print(f'Copied/archived {index + 1}/{len(files)} files; {total_bytes / 2**30:.2f} GiB of source data', flush=True)
                last_update = time.monotonic()
    print('Verifying every archived executable by SHA-256', flush=True)
    with zipfile.ZipFile(zip_path) as archive:
        for sha, entry in objects.items():
            with archive.open(entry) as stream:
                assert hashlib.file_digest(stream, 'sha256').hexdigest() == sha, 'archive checksum differs'
            if time.monotonic() - last_update >= 25:
                print('Executable archive verification continues', flush=True)
                last_update = time.monotonic()
    document_records = []
    for source in documents:
        target = destination / 'documents' / source.name
        shutil.copyfile(source, target)
        sha = digest(source)
        assert digest(target) == sha
        document_records.append({'source': str(source), 'destination': str(target), 'sha256': sha})
    # Recheck the entire source set before removing any original working copy.
    assert set(files) == {p for p in CAMPAIGN.rglob('*') if p.is_file()}
    for source, metadata in originals.items():
        assert (source.stat().st_size, source.stat().st_mtime_ns) == metadata, 'source changed before switch'
    for record in document_records:
        assert digest(Path(record['source'])) == record['sha256']
    receipt = {
        'format': 'dialect-progress-relocation-v1', 'source_campaign': str(CAMPAIGN),
        'artifact_campaign': str(artifacts), 'repository': str(destination),
        'source_bytes': total_bytes, 'files': records, 'documents': document_records,
        'executable_archive': str(zip_path), 'executable_archive_sha256': digest(zip_path),
        'unique_archived_executables': len(objects), 'kept_executables': sorted(KEEP_EXECUTABLES),
        'verification': 'every copied file and every unique archived executable checked by SHA-256; source metadata stable',
        'source_git_head': subprocess.check_output(['git', '-C', str(ROOT), 'rev-parse', 'HEAD'], text=True).strip(),
    }
    receipt_path = destination / 'logs/storage_migration_20261018.json'
    receipt_path.write_text(json.dumps(receipt, indent=2) + '\n')
    (destination / 'README.md').write_text('''# Imscribing progress records

Progress documents and human logs live here on D:. Large state datasets and
build artifacts live under `artifacts/`, outside Git tracking. Historical ELF
executables are stored once per content hash in a verified ZIP archive.

The original WSL campaign path links to the artifact directory. Small receipts
and logs inside that directory link to their canonical files under `logs/`.
The progress write-up in `documents/` is also linked from its original ig-docs path.

The active executable is `artifacts/dialect_transport_20261017/locked_512_r23`.
Its full-coordinate preparation replay and payload checks precede its first
production run. The latest completed production return is locked r22.

`logs/storage_migration_20261018.json` maps every original file to its canonical
stored file or ZIP entry and records its exact SHA-256. The old repository's
Git history remains intact. No remote publication is configured.
''')
    restore = destination / 'restore_executable.py'
    restore.write_text('''#!/usr/bin/env python3
"""Restore one archived executable to its original campaign-relative path."""
import hashlib,json,os,sys,zipfile
from pathlib import Path
root=Path(__file__).resolve().parent
manifest=json.loads((root/'logs/storage_migration_20261018.json').read_text())
record=next(r for r in manifest['files'] if r['path']==sys.argv[1])
assert record['storage']=='executable-zip'
target=Path(manifest['artifact_campaign'])/record['path']
assert not target.exists()
with zipfile.ZipFile(manifest['executable_archive']) as archive:
    with archive.open(record['entry']) as source,target.open('wb') as output:
        while chunk:=source.read(8*1024*1024):output.write(chunk)
with target.open('rb') as source:assert hashlib.file_digest(source,'sha256').hexdigest()==record['sha256']
try:os.chmod(target,record['mode'])
except PermissionError:assert os.access(target,os.X_OK)
print(target)
''')
    if args.stage_only:
        receipt['original_working_copies_removed'] = False
        receipt['stage_only'] = True
        receipt_path.write_text(json.dumps(receipt, indent=2) + '\n')
        readme = destination / 'README.md'
        text = readme.read_text().replace('The original WSL campaign path links to the artifact directory.', 'The D: copy is verified. Switching the original WSL campaign path is deferred until the concurrent source cleanup finishes.')
        readme.write_text(text)
        commit_repository(destination, 'I preserve my progress documents and executable archives on D')
        print('D: staging verified and committed; all source paths left unchanged', flush=True)
        return
    backup = CAMPAIGN.with_name(CAMPAIGN.name + '.verified-migration-original')
    assert not backup.exists()
    CAMPAIGN.rename(backup)
    try:
        CAMPAIGN.symlink_to(artifacts, target_is_directory=True)
    except BaseException:
        backup.rename(CAMPAIGN)
        raise
    for record in document_records:
        source = Path(record['source'])
        temporary = source.with_name('.' + source.name + '.D-link')
        temporary.symlink_to(record['destination'])
        temporary.replace(source)
    print('All bytes verified; removing original WSL working copies', flush=True)
    shutil.rmtree(backup)
    receipt['original_working_copies_removed'] = True
    receipt_path.write_text(json.dumps(receipt, indent=2) + '\n')
    commit_repository(destination, 'I move my progress records to D and archive my historical executables')
    print(json.dumps({'repository': str(destination), 'source_GiB_relocated': total_bytes / 2**30,
                      'executable_archive_GiB': zip_path.stat().st_size / 2**30,
                      'unique_archived_executables': len(objects), 'original_copies_removed': True}), flush=True)


if __name__ == '__main__':
    main()
