"""Read packed continuation cells without expanding Python tuple arenas."""
from array import array
import gzip
import struct
import sys
import mmap
import hashlib
import json
from pathlib import Path


def mapped(path):
    with path.open('rb') as stream:
        return mmap.mmap(stream.fileno(),0,access=mmap.ACCESS_READ) if path.stat().st_size else b''


def file_digest(path):
    with path.open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()


class PackedRecords:
    def __init__(self,path,format):
        self.raw=mapped(path);self.format=format;self.width=struct.calcsize(format)
        if len(self.raw)%self.width:raise ValueError('partial source relation record')
    def __len__(self):return len(self.raw)//self.width
    def __iter__(self):return struct.iter_unpack(self.format,self.raw)
    def __getitem__(self,index):
        if isinstance(index,slice):return [self[i] for i in range(*index.indices(len(self)))]
        if index<0:index+=len(self)
        if not 0<=index<len(self):raise IndexError(index)
        return struct.unpack_from(self.format,self.raw,index*self.width)


class PackedTrail:
    def __init__(self,raw):
        if len(raw)%9:raise ValueError('partial packed continuation cell')
        self.raw=raw
    def __len__(self):return len(self.raw)//9
    def __iter__(self):return struct.iter_unpack('<QB',self.raw)
    def __reversed__(self):
        return (struct.unpack_from('<QB',self.raw,i) for i in range(len(self.raw)-9,-1,-9))
    def __getitem__(self,index):
        if isinstance(index,slice):
            start,stop,step=index.indices(len(self))
            if step==1:return PackedTrail(self.raw[9*start:9*stop])
            return [self[i] for i in range(start,stop,step)]
        if index<0:index+=len(self)
        if not 0<=index<len(self):raise IndexError(index)
        return struct.unpack_from('<QB',self.raw,index*9)


class ExternalRecords:
    """Lazily read the immutable prefix named by a resident checkpoint."""
    def __init__(self,descriptor,transform=None):
        if descriptor['encoding']!='length-u64le-gzip-json':raise ValueError('unknown evidence encoding')
        self.path=Path(descriptor['external_archive']);self.transform=transform
        self.offsets=array('Q');self.lengths=array('Q');boundary=descriptor['bytes'];offset=0
        with self.path.open('rb') as stream:
            if self.path.stat().st_size<boundary:raise ValueError('truncated retained evidence')
            for _ in range(descriptor['records']):
                header=stream.read(8)
                if len(header)!=8:raise ValueError('partial evidence header')
                length=struct.unpack('<Q',header)[0]
                if offset+8+length>boundary:raise ValueError('evidence record crosses checkpoint boundary')
                self.offsets.append(offset+8);self.lengths.append(length);offset+=8+length;stream.seek(offset)
            if offset!=boundary:raise ValueError('evidence prefix record count changed')
    def __len__(self):return len(self.offsets)
    def __getitem__(self,index):
        if isinstance(index,slice):return [self[i] for i in range(*index.indices(len(self)))]
        if index<0:index+=len(self)
        if not 0<=index<len(self):raise IndexError(index)
        with self.path.open('rb') as stream:
            stream.seek(self.offsets[index]);encoded=stream.read(self.lengths[index])
        value=json.loads(gzip.decompress(encoded))
        return self.transform(value) if self.transform else value
    def __iter__(self):return (self[i] for i in range(len(self)))


def archived(value,transform=None):
    if isinstance(value,dict) and 'external_archive' in value:return ExternalRecords(value,transform)
    return [transform(item) for item in value] if transform else value


def unpack(report):
    if 'register_banks' in report:
        transform=(lambda item:[item[0],bytes(item[1]).hex()]) if report.get('register_banks_encoding')=='gzip-bytes' else None
        report['register_banks']=archived(report['register_banks'],transform)
        if transform:report['register_banks_encoding']='gzip-hex'
    if 'nested_receipts' in report:report['nested_receipts']=archived(report['nested_receipts'])
    if 'reversal_receipts' in report:report['reversal_receipts']=archived(report['reversal_receipts'])
    if report.get('source_product_return'):
        report['source_product_return']['banks']=archived(report['source_product_return']['banks'])
    c=report['continuation']
    if c.get('trail_gzip_hex') is not None:
        c['trail']=PackedTrail(gzip.decompress(bytes.fromhex(c.pop('trail_gzip_hex'))))
        if len(c['trail'])!=c['trail_length']:raise ValueError('packed trail count changed')
    if c.get('pending_gates_gzip_hex') is not None:
        raw=gzip.decompress(bytes.fromhex(c.pop('pending_gates_gzip_hex')))
        if len(raw)%8:raise ValueError('partial packed queue cell')
        values=array('Q');values.frombytes(raw)
        if sys.byteorder!='little':values.byteswap()
        if len(values)!=c['pending_gates_length']:raise ValueError('packed queue count changed')
        c['pending_gates']=values
    return report
