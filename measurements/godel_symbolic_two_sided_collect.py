"""Let correction copies update either operand of a signed product."""
from godel_markdown_records import read_record, write_record
from godel_symbolic_residual_absorption import absorb
from godel_symbolic_boundary_collect import value


def collect(A,B,R,left_first):
    A,B,R=dict(A),dict(B),dict(R)
    moves=[]
    initial=value(A)*value(B)+value(R)
    while True:
        changed=False
        for side in (('A','B') if left_first else ('B','A')):
            payload,placement=(A,B) if side=='A' else (B,A)
            if not payload:
                continue
            updated,residual,copies=absorb(payload,placement,R)
            if not copies:
                continue
            if side=='A':
                B=updated
            else:
                A=updated
            R=residual
            changed=True
            moves.append(dict(payloadSide=side,copies=copies,
                operandA=sorted(A.items()),operandB=sorted(B.items()),remainingTerms=len(R)))
            assert value(A)*value(B)+value(R)==initial
        if not changed:
            return A,B,R,moves


def main():
    parents=read_record('godel_symbolic_overlap_absorption')['statesWithAbsorption']
    reports=[]
    checked=0
    for index,row in enumerate(parents):
        for left_first in (False,True):
            checked+=1
            A,B,R=(dict(row[k]) for k in ('payload','placement','correction'))
            a,b,r,moves=collect(A,B,R,left_first)
            assert value(a)*value(b)+value(r)==int(row['source'])
            if not moves:
                continue
            closed=not r and abs(value(a))>1 and abs(value(b))>1
            reports.append(dict(parentRecord='godel_symbolic_overlap_absorption',parentState=index,
                source=row['source'],firstPayload='A' if left_first else 'B',moves=moves,
                operandA=sorted(a.items()),operandB=sorted(b.items()),correction=sorted(r.items()),
                exactReturnEvidence='True',sourceFactorEvidence='True' if closed else 'None'))
    write_record('godel_symbolic_two_sided_collect',dict(checkedStates=checked,
        retention='Changed states retained; unchanged shapes are referenced parent records',reports=reports))
    print('Checked:',checked,'changed:',len(reports),'closed:',sum(r['sourceFactorEvidence']=='True' for r in reports))
    for n in sorted(set(r['source'] for r in reports),key=int):
        rows=[r for r in reports if r['source']==n]
        print('Source:',n,'changed:',len(rows),'minimum residual:',min(len(r['correction']) for r in rows),
            'maximum collection passes:',max(len(r['moves']) for r in rows))


if __name__=='__main__':
    main()
