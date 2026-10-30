"""File-backed, input-bound decoded banks for the independent continuation audit."""
from array import array
import fcntl
import hashlib
import json
import os
from pathlib import Path
import struct
import sys

from packed_relational_return import mapped, file_digest

MAGIC=b'RBANK01\0'
HEADER=struct.Struct('<8sQQ')


def json_digest(value):
    hashed=hashlib.sha256()
    for block in json.JSONEncoder(sort_keys=True,separators=(',',':')).iterencode(value):
        hashed.update(block.encode())
    return hashed.hexdigest()


def input_binding(description,prepared,bits,validator):
    archive_digest=None
    if isinstance(description,dict) and 'external_archive' in description:
        remaining=description['bytes'];hashed=hashlib.sha256()
        with Path(description['external_archive']).open('rb') as stream:
            while remaining:
                block=stream.read(min(1024*1024,remaining))
                if not block:raise ValueError('truncated audit bank prefix')
                hashed.update(block);remaining-=len(block)
        archive_digest=hashed.hexdigest()
    try:root=memoryview(prepared)
    except TypeError:root=bytes(prepared)
    return {'format':'resident-bank-audit-input-v1','bank_descriptor_sha256':json_digest(description),
            'archive_prefix_sha256':archive_digest,'prepared_root_sha256':hashlib.sha256(root).hexdigest(),
            'bank_value_bits':bits,'byteorder':sys.byteorder,'validator_sha256':file_digest(validator),
            'packed_reader_sha256':file_digest(Path(__file__).with_name('packed_relational_return.py')),
            'store_sha256':file_digest(Path(__file__))}


class MappedBankStore:
    """Append a bank only AFTER checking its source contractions and ancestry.

    Each committed bank is checksummed and mapped read-only. A resumed audit
    accepts a decoded prefix only for the exact source, root and validator.
    No executing alternatives are truncated or evicted from the bank family.
    """
    def __init__(self,directory,binding):
        self.directory=Path(directory);self.directory.mkdir(parents=True,exist_ok=True)
        self.guard=(self.directory/'writer.lock').open('a+b')
        try:fcntl.flock(self.guard,fcntl.LOCK_EX|fcntl.LOCK_NB)
        except BlockingIOError:raise ValueError('this decoded bank audit is already active') from None
        self.binding=binding;self.manifest=self.directory/'index.json';self.entries=[];self.maps=[]
        self.parents=array('Q',[0]);self.keys=[memoryview(b'').cast('Q')];self.values=[memoryview(b'')]
        if self.manifest.exists():
            saved=json.loads(self.manifest.read_text())
            if saved.get('format')!='resident-bank-audit-store-v1' or saved['input']!=binding:
                raise ValueError('decoded bank cache changed its input or validator binding')
            for row in saved['banks']:
                self._map(row);self.entries.append(row)
        self.reused=len(self.entries)

    def __del__(self):
        guard=getattr(self,'guard',None)
        if guard is not None:guard.close()

    def _map(self,row):
        bank=len(self.parents);path=self.directory/f'{bank:08d}.bank'
        if row['bank']!=bank or not 0<=row['parent']<bank or row['cells']<0:
            raise ValueError('decoded bank cache changed its ancestry or ordinal')
        if file_digest(path)!=row['sha256']:
            raise ValueError('decoded bank cache content changed')
        raw=mapped(path)
        if len(raw)<HEADER.size:raise ValueError('decoded bank cache has a partial header')
        magic,parent,count=HEADER.unpack_from(raw)
        if (magic,parent,count)!=(MAGIC,row['parent'],row['cells']) or len(raw)!=HEADER.size+9*count:
            raise ValueError('decoded bank cache changed its layout')
        self.maps.append(raw);self.parents.append(parent)
        end=HEADER.size+8*count
        self.keys.append(memoryview(raw)[HEADER.size:end].cast('Q'))
        self.values.append(memoryview(raw)[end:])

    def append(self,parent,wires,values):
        bank=len(self.parents)
        if not 0<=parent<bank or len(wires)!=len(values):
            raise ValueError('invalid independently checked bank append')
        if wires.typecode!='Q':wires=array('Q',wires)
        target=self.directory/f'{bank:08d}.bank'
        staging=self.directory/f'{bank:08d}.{os.getpid()}.next'
        hashed=hashlib.sha256()
        with staging.open('wb') as stream:
            for block in (HEADER.pack(MAGIC,parent,len(wires)),wires,values):
                stream.write(block);hashed.update(block)
            stream.flush();os.fsync(stream.fileno())
        sha=hashed.hexdigest()
        if target.exists():
            if file_digest(target)!=sha:raise ValueError('uncommitted decoded bank differs from its source')
            staging.unlink()
        else:staging.replace(target)
        row={'bank':bank,'parent':parent,'cells':len(wires),'sha256':sha}
        self._map(row);self.entries.append(row)
        staging_index=self.directory/f'index.{os.getpid()}.next'
        with staging_index.open('w') as stream:
            json.dump({'format':'resident-bank-audit-store-v1','input':self.binding,'banks':self.entries},stream)
            stream.write('\n');stream.flush();os.fsync(stream.fileno())
        staging_index.replace(self.manifest)

    def receipt(self):
        return {'kind':'mapped','directory':str(self.directory.resolve()),
                'input_sha256':json_digest(self.binding),'decoded_banks':len(self.entries),
                'reused_decoded_banks':self.reused,'decoded_this_run':len(self.entries)-self.reused,
                'cells':sum(row['cells'] for row in self.entries)}
