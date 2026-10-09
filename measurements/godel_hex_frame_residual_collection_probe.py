"""Collect literal operand copies from the six retained signed corrections."""
import gzip
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
DEST = ROOT / 'measurements/exact_hex_target'
EMPTY, FILLED = '≻⋈∈⊤∋', '≻⋈∈⊥∋'
SOURCE = (DEST / 'source.txt').read_text().strip()


def field(out, label):
    return next(line.split(None, 1)[1] for line in out.splitlines()
                if line.startswith(label + ' '))


def cells(word):
    return [word[1:-3][i:i + 5] for i in range(0, len(word[1:-3]), 5)]


def main():
    relations = []
    with gzip.open(DEST / 'frame_sum_returns.log.gz', 'rt') as stream:
        for line in stream:
            match = re.match(r'SIGNED SOURCE RETURN (\d+) = ([+-])(\d+) \* (\d+) - \(([+-])(\d+)\)', line)
            if match:
                assert match[1] == SOURCE
                relations.append(match.groups()[1:])
    assert len(relations) == 6
    with gzip.open(DEST / 'frame_residual_collection.log.gz', 'wt') as log:
        def run(args):
            result = subprocess.run(args, cwd=ROOT, capture_output=True, text=True)
            log.write('COMMAND ' + repr(args) + '\n' + result.stdout + result.stderr +
                      '\nEXIT ' + str(result.returncode) + '\n')
            assert result.returncode == 0, result.stdout + result.stderr
            return result.stdout

        def word(value):
            return field(run(['./godel', 'encode', value]), 'word')

        def decode(parts):
            return field(run(['./godel', 'decode', '⊢' + ''.join(parts) + '⊙⊡⊣']), 'value')

        def operate(a, op, b):
            if b == '0' and op in {'add', 'sub', 'mul'}:
                value = '0' if op == 'mul' else a
                certificate(a, 'mul' if op == 'mul' else 'add', '0', value)
                return 'result ' + value + '\n'
            left, right = cells(word(a)), cells(word(b))
            width = max(2, len(left), len(right))
            carrier = decode(left + [EMPTY] * (2 * width - len(left)) + right)
            return run(['./godel', 'frame-op', carrier, str(width), '0', op,
                        str(2 * width), '1'])

        def certificate(a, op, b, value):
            result = run(['./godel', 'check', op, word(a), word(b), word(value)])
            assert 'PASS' in result and 'FAIL' not in result

        summaries = []
        for index, (product_sign, left, right, residual_sign, residual) in enumerate(relations):
            parts = cells(word(residual))
            shift = parts.index(FILLED)
            scale, tail = decode([EMPTY] * shift + [FILLED]), decode(parts[shift:])
            certificate(scale, 'mul', tail, residual)
            for side, operand, other in [('left', left, right), ('right', right, left)]:
                out = operate(tail, 'divmod', operand)
                quotient, remainder = field(out, 'result'), field(out, 'remainder')
                copies = field(operate(operand, 'mul', quotient), 'result')
                certificate(operand, 'mul', quotient, copies)
                certificate(copies, 'add', remainder, tail)
                coefficient = field(operate(scale, 'mul', quotient), 'result')
                positioned_remainder = field(operate(scale, 'mul', remainder), 'result')
                # Reconstitute the signed cofactor entirely through native words.
                if product_sign == residual_sign:
                    a, b = (other, coefficient) if product_sign == '+' else (coefficient, other)
                    # The retained sign comparison is read from the exact words.
                    ap, bp = cells(word(a)), cells(word(b))
                    abits, bbits = tuple(x == FILLED for x in reversed(ap)), tuple(x == FILLED for x in reversed(bp))
                    order = (len(ap), abits) >= (len(bp), bbits)
                    cofactor_sign = '+' if order else '-'
                    cofactor = field(operate(a if order else b, 'sub', b if order else a), 'result')
                else:
                    cofactor_sign = product_sign
                    cofactor = field(operate(other, 'add', coefficient), 'result')
                magnitude = field(operate(operand, 'mul', cofactor), 'result')
                certificate(operand, 'mul', cofactor, magnitude)
                if cofactor_sign == '+':
                    if residual_sign == '+':
                        certificate(SOURCE, 'add', positioned_remainder, magnitude)
                    else:
                        certificate(magnitude, 'add', positioned_remainder, SOURCE)
                else:
                    assert residual_sign == '-'
                    certificate(SOURCE, 'add', magnitude, positioned_remainder)
                run(['./run_cmds.sh', 'tfactor read ' + remainder])
                summary = f'Return {index + 1}, {side}: payload copies `{quotient}`, remainder `{remainder}`.\n\n`{SOURCE} = {cofactor_sign}{operand} * {cofactor} - ({residual_sign}{positioned_remainder})`\n'
                summaries.append(summary)
                log.write('COLLECTED SOURCE RETURN ' + summary + '\n')
                print(f'return={index + 1} side={side} exact-collection={remainder == "0"} source-equation=PASS', flush=True)
                if remainder == '0' and cofactor_sign == '+' and operand not in {'0', '1', SOURCE} and cofactor not in {'0', '1', SOURCE}:
                    proof = run(['./godel', 'product', SOURCE, operand, cofactor])
                    assert 'relation.exact-product     PASS' in proof
                    print('FACTORS', operand, cofactor, flush=True)
        (DEST / 'frame_residual_collection.md').write_text(
            '# Signed correction collection\n\n'
            'Each correction is split into its initial empty run and payload. '
            'Gödel reads exact copies of each exposed operand from that payload and retains the remainder. '
            'Every resulting signed source equation is checked independently.\n\n' + '\n'.join(summaries))


if __name__ == '__main__':
    main()
