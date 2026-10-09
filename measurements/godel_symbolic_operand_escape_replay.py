"""Replay recorded operand transport and descent with per-move return checks."""
from godel_markdown_records import read_record, write_record
from godel_symbolic_operand_escape import transport
from godel_symbolic_zero_escape import magnitude
from godel_symbolic_boundary_collect import value


def main():
    parents = read_record('godel_symbolic_zero_escape')['reports']
    rows = read_record('godel_symbolic_operand_escape')['reports']
    checked = 0
    for row in rows:
        parent = parents[row['parentState']]
        A,B,R = (dict(parent[k]) for k in ('operandA','operandB','correction'))
        n = int(row['source'])
        for round_ in row['rounds']:
            assert magnitude(R) == round_['beforeMagnitude']
            A,B,R = transport(A,B,R,round_['operandSide'],*round_['polarityRewrite'])
            assert value(A)*value(B)+value(R) == n
            for operation in round_['descent']:
                if 'collection' in operation:
                    move = operation['collection']
                    assert magnitude(R) == move['beforeMagnitude']
                    payload = A if move['payloadSide'] == 0 else B
                    placement = B if move['payloadSide'] == 0 else A
                    for i,c in payload.items():
                        j = i + move['shift']
                        R[j] = R.get(j,0) - move['sign']*c
                    R = {i:c for i,c in R.items() if c}
                    j = move['shift']
                    placement[j] = placement.get(j,0) + move['sign']
                    placement = {i:c for i,c in placement.items() if c}
                    if move['payloadSide'] == 0:
                        B = placement
                    else:
                        A = placement
                    assert R == dict(move['correction'])
                    assert magnitude(R) == move['afterMagnitude'] < move['beforeMagnitude']
                    assert value(A)*value(B)+value(R) == n
                    checked += 1
                else:
                    for kind,i,sign in operation['cancellations']:
                        before = magnitude(R)
                        if kind == 'merge':
                            assert R[i]*sign >= 2
                            R[i] -= 2*sign
                            R[i+1] = R.get(i+1,0)+sign
                        else:
                            assert kind == 'split-cancel' and R[i]*sign < 0 and R[i+1]*sign > 0
                            R[i+1] -= sign
                            R[i] += 2*sign
                        R = {j:c for j,c in R.items() if c}
                        assert magnitude(R) < before
                        assert value(A)*value(B)+value(R) == n
                        checked += 1
            assert magnitude(R) == round_['afterMagnitude'] < round_['beforeMagnitude']
        assert [dict(row[k]) for k in ('operandA','operandB','correction')] == [A,B,R]
    write_record('godel_symbolic_operand_escape_replay',dict(replayedStates=len(rows),
        verifiedDescentMoves=checked,verification='Replay every accepted transport, collection and individual cancellation; assert source return and recorded endpoints'))
    print('Replayed states:',len(rows),'verified descent moves:',checked)


if __name__ == '__main__':
    main()
