"""Check immutable dependency incidence against every source predicate port."""
import struct
import sys
from packed_relational_return import mapped,file_digest


def audit(directory,report,binding,ops,cells,altered_raw=None):
    if not report.get('prepared_dependency_incidence'):return {}
    path=directory/'reader.index'
    if file_digest(path)!=binding['correlated_artifacts']['reader.index']:
        raise ValueError('prepared dependency incidence changed')
    raw=mapped(path) if altered_raw is None else altered_raw
    wires=struct.unpack_from('<Q',raw)[0]
    if wires!=len(ops) or sys.byteorder!='little':raise ValueError('prepared reader index host/source geometry differs')
    at=8+8*(wires+1)
    if len(raw)<at or (len(raw)-at)%8:raise ValueError('prepared reader index length differs')
    offsets=memoryview(raw)[8:at].cast('Q');readers=memoryview(raw)[at:].cast('Q')
    expected=sum(2 if op==3 else 3 if op in (4,7) else 0 for op,_,_,_ in ops)+5*len(cells)
    if offsets[0]!=0 or offsets[-1]!=len(readers) or len(readers)!=expected:
        raise ValueError('prepared reader index omitted or added incidences')
    for wire in range(wires):
        lo,hi=offsets[wire],offsets[wire+1]
        if not 0<=lo<=hi<=len(readers):raise ValueError('prepared reader offsets changed')
        previous=None;copies=0
        for index in range(lo,hi):
            gate=readers[index]
            if previous is not None and gate<previous:raise ValueError('prepared reader order changed')
            copies=copies+1 if gate==previous else 1;previous=gate
            if gate<len(ops):
                op,a,b,_=ops[gate]
                if op not in (3,4,7):raise ValueError('prepared reader is not a predicate')
                multiplicity=int(a==wire)+int(op!=3 and b==wire)+int(gate==wire)
            elif gate<len(ops)+len(cells):multiplicity=cells[gate-len(ops)].count(wire)
            else:raise ValueError('prepared reader escaped source predicates')
            if copies>multiplicity:raise ValueError('prepared reader port or repeated incidence differs')
    return {'prepared_reader_incidences_checked':expected}
