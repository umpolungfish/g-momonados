"""Read packed continuation cells without expanding Python tuple arenas."""
from array import array
import gzip
import struct
import sys


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


def unpack(report):
    c=report['continuation']
    if c.get('trail_gzip_hex') is not None:
        c['trail']=PackedTrail(gzip.decompress(bytes.fromhex(c['trail_gzip_hex'])))
        if len(c['trail'])!=c['trail_length']:raise ValueError('packed trail count changed')
    if c.get('pending_gates_gzip_hex') is not None:
        raw=gzip.decompress(bytes.fromhex(c['pending_gates_gzip_hex']))
        if len(raw)%8:raise ValueError('partial packed queue cell')
        values=array('Q');values.frombytes(raw)
        if sys.byteorder!='little':values.byteswap()
        if len(values)!=c['pending_gates_length']:raise ValueError('packed queue count changed')
        c['pending_gates']=values
    return report
