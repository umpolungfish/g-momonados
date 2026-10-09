"""Transport local operand zero rewrites before residual collection descent."""
from godel_markdown_records import read_record, write_record
from godel_symbolic_zero_escape import descend, magnitude
from godel_symbolic_residual_absorption import product
from godel_symbolic_boundary_collect import value


def transport(A, B, R, side, position, sign):
    a, b, r = dict(A), dict(B), dict(R)
    operand = a if side == 0 else b
    operand[position] = operand.get(position, 0) - 2 * sign
    operand[position + 1] = operand.get(position + 1, 0) + sign
    operand = {i:c for i,c in operand.items() if c}
    difference = {position:-2 * sign, position + 1:sign}
    for i,c in product(difference, B if side == 0 else A).items():
        r[i] = r.get(i, 0) - c
    r = {i:c for i,c in r.items() if c}
    a, b = (operand, b) if side == 0 else (a, operand)
    assert value(a) == value(A) and value(b) == value(B)
    assert value(r) == value(R)
    return a, b, r


def main():
    # The transported signed shape differs, while its numeric return stays fixed.
    control = transport({0:1, 1:1}, {2:1}, {0:1, 1:2, 2:1}, 0, 0, 1)
    assert value(control[0])*value(control[1])+value(control[2]) == 21
    ca,cb,cr,cm = descend(*control, False, False)
    assert value(ca)*value(cb)+value(cr) == 21
    parents = read_record('godel_symbolic_zero_escape')['reports']
    reports = []
    for index,parent in enumerate(parents):
        A,B,R = (dict(parent[k]) for k in ('operandA','operandB','correction'))
        n = int(parent['source'])
        checked = 0
        rounds = []
        while True:
            score = magnitude(R)
            best = None
            for side,operand in enumerate((A,B)):
                for position,c in sorted(operand.items()):
                    sign = 1 if c > 0 else -1
                    transported = transport(A,B,R,side,position,sign)
                    for reverse in (False,True):
                        for higher in (False,True):
                            checked += 1
                            a,b,r,moves = descend(*(dict(t) for t in transported),reverse,higher)
                            assert value(a)*value(b)+value(r) == n
                            final = magnitude(r)
                            if final >= score or (best is not None and final >= best[0]):
                                continue
                            best = (final,a,b,r,dict(operandSide=side,polarityRewrite=[position,sign],
                                shiftReverse=reverse,cancellationHigherFirst=higher,descent=moves,
                                beforeMagnitude=score,afterMagnitude=final))
            if best is None:
                break
            _,A,B,R,move = best
            rounds.append(move)
        closed = not R and abs(value(A)) > 1 and abs(value(B)) > 1
        reports.append(dict(source=str(n),parentState=index,checkedPaths=checked,rounds=rounds,
            operandA=sorted(A.items()),operandB=sorted(B.items()),correction=sorted(R.items()),
            exactReturnEvidence='True',sourceFactorEvidence='True' if closed else 'None'))
        print('Source:',n,'parent:',index,'paths:',checked,'accepted:',len(rounds),
              'terms:',len(R),'magnitude:',magnitude(R),'factor evidence:',reports[-1]['sourceFactorEvidence'],flush=True)
    write_record('godel_symbolic_operand_escape',dict(parentRecord='godel_symbolic_zero_escape',
        control=dict(source='21',transported=[sorted(t.items()) for t in control],
                     descent=cm,final=[sorted(t.items()) for t in (ca,cb,cr)]),reports=reports))


if __name__ == '__main__':
    main()
