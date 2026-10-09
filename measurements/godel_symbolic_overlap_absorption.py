"""Collect correction copies exposed by overlapping signed products."""
from godel_markdown_records import read_record, write_record
from godel_symbolic_residual_absorption import absorb
from godel_symbolic_rewrite_absorption import merge
from godel_symbolic_boundary_collect import value


def main():
    prior = read_record('godel_symbolic_overlap_collections')
    successes = []
    checked = 0
    parents = 0
    for report in prior['reports']:
        n = int(report['source'])
        for index, state in enumerate(report['states']):
            for swap in (False, True):
                parents += 1
                A, B = dict(state['payload']), dict(state['placement'])
                if swap:
                    A, B = B, A
                anchor = min(A)
                A = {i-anchor: c for i,c in A.items()}
                B = {i+anchor: c for i,c in B.items()}
                R = dict(state['correction'])
                shapes = [(None, R, [])]
                for i,c in sorted(R.items()):
                    if abs(c) != 1:
                        continue
                    t = dict(R)
                    t[i] -= 2*c
                    t[i+1] = t.get(i+1,0)+c
                    t, moves = merge(t)
                    shapes.append(([i,c], t, moves))
                for rewrite, residual, merges in shapes:
                    checked += 1
                    assert value(residual) == value(R)
                    newB, newR, moves = absorb(A,B,residual)
                    assert value(A)*value(newB)+value(newR) == n
                    if not moves:
                        continue
                    closed = not newR and abs(value(A))>1 and abs(value(newB))>1
                    successes.append(dict(source=str(n),parentState=index,swap=swap,
                        payload=sorted(A.items()),initialPlacement=sorted(B.items()),
                        initialCorrection=sorted(R.items()),polarityRewrite=rewrite,
                        unitMerges=merges,absorptions=moves,placement=sorted(newB.items()),
                        correction=sorted(newR.items()),exactReturnEvidence='True',
                        sourceFactorEvidence='True' if closed else 'None'))
    write_record('godel_symbolic_overlap_absorption',dict(parentStates=parents,
        checkedEquivalentShapes=checked,statesWithAbsorption=successes))
    print('Parent states:',parents,'Equivalent shapes:',checked)
    print('Absorptions:',len(successes),'Whole returns:',sum(s['sourceFactorEvidence']=='True' for s in successes))
    for n in sorted(set(s['source'] for s in successes),key=int):
        rows=[s for s in successes if s['source']==n]
        print('Source:',n,'absorptions:',len(rows),'minimum correction terms:',min(len(s['correction']) for s in rows))


if __name__ == '__main__':
    main()
