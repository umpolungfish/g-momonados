"""Stable bounded log labels; the exact source remains in every command record."""
import hashlib
import gzip
from pathlib import Path

def record_name(source):
    return source if len(source) <= 120 else "source_" + hashlib.sha256(source.encode()).hexdigest()[:20]

def read_record(path):
    path = Path(path)
    archive = Path(str(path) + '.gz')
    history = ''
    if archive.exists():
        with gzip.open(archive, 'rt') as stream:
            history = stream.read()
    return history + (path.read_text() if path.exists() else '')
