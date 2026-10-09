"""Read signed source corrections from the retained full-frame root words.

Python selects recorded words and orchestrates the two research instruments.
Every numeral operation and equation certificate is produced by godel.
"""
import gzip
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
DEST = ROOT / 'measurements' / 'exact_hex_target'
EMPTY = '≻⋈∈⊤∋'
FILLED = '≻⋈∈⊥∋'
SOURCE = (DEST / 'source.txt').read_text().strip()


def field(out, label):
    return next(line.split(None, 1)[1] for line in out.splitlines()
                if line.startswith(label + ' '))


def cells(word):
    return [word[1:-3][i:i + 5] for i in range(0, len(word[1:-3]), 5)]


def main():
    from godel_hex_record_name import record_name
    prior = ROOT / 'measurements' / (
        'godel_hex_frame_sum_' + record_name(SOURCE) + '_low_sum.log.gz')
    returns = []
    with gzip.open(prior, 'rt') as stream:
        for line in stream:
            match = re.match(r'SOURCE FRAME SUM width=(\d+) sum=(\d+)', line)
            if match:
                width, selected_sum = match.groups()
            match = re.match(r'FRAME SUM RETAINED ROOT WORDS (\d+) (\d+)', line)
            if match:
                returns.extend((width, selected_sum, root) for root in match.groups())
    assert len(returns) == 6, 'Require both complete returns at all three boundaries'
    with gzip.open(DEST / 'frame_sum_returns.log.gz', 'wt') as log:
        def run(args, underflow=False):
            result = subprocess.run(args, cwd=ROOT, capture_output=True, text=True)
            log.write('COMMAND ' + repr(args) + '\n' + result.stdout + result.stderr +
                      '\nEXIT ' + str(result.returncode) + '\n')
            if result.returncode:
                if underflow and 'frame subtraction underflow' in result.stdout + result.stderr:
                    return None
                raise RuntimeError(result.stdout + result.stderr)
            return result.stdout

        def word(value):
            return field(run(['./godel', 'encode', value]), 'word')

        def decode(parts):
            return field(run(['./godel', 'decode', '⊢' + ''.join(parts) + '⊙⊡⊣']), 'value')

        def operate(a, op, b):
            left, right = cells(word(a)), cells(word(b))
            width = max(2, len(left), len(right))
            carrier = decode(left + [EMPTY] * (2 * width - len(left)) + right)
            return run(['./godel', 'frame-op', carrier, str(width), '0', op,
                        str(2 * width), '1'], underflow=op == 'sub')

        def certificate(a, op, b, result):
            output = run(['./godel', 'check', op, word(a), word(b), word(result)])
            assert 'PASS' in output and 'FAIL' not in output

        summaries = []
        # Control checks the equation checker accepts the source identity and
        # refuses changing its complete output by one native unit.
        certificate(SOURCE, 'mul', '1', SOURCE)
        changed = field(operate(SOURCE, 'add', '1'), 'result')
        rejected = subprocess.run(['./godel', 'check', 'mul', word(SOURCE), word('1'),
                                   word(changed)], cwd=ROOT, capture_output=True, text=True)
        log.write('CONTROL altered source\n' + rejected.stdout + rejected.stderr)
        assert 'FAIL' in rejected.stdout
        for width, selected_sum, root in returns:
            half = operate(selected_sum, 'divmod', '2')
            assert field(half, 'remainder') == '0'
            half = field(half, 'result')
            right = field(operate(half, 'add', root), 'result')
            certificate(half, 'add', root, right)
            difference = operate(half, 'sub', root)
            sign = '+' if difference is not None else '-'
            left = field(difference if difference is not None else operate(root, 'sub', half), 'result')
            certificate(left, 'add', root, half) if sign == '+' else certificate(left, 'add', half, root)
            product = field(operate(left, 'mul', right), 'result')
            certificate(left, 'mul', right, product)
            if sign == '-':
                residual = field(operate(product, 'add', SOURCE), 'result')
                residual_sign = '-'
                certificate(product, 'add', SOURCE, residual)
            else:
                difference = operate(product, 'sub', SOURCE)
                residual_sign = '+' if difference is not None else '-'
                residual = field(difference if difference is not None else operate(SOURCE, 'sub', product), 'result')
                certificate(SOURCE, 'add', residual, product) if difference is not None else certificate(product, 'add', residual, SOURCE)
            parts = cells(word(residual))
            first = next((i for i, cell in enumerate(parts) if cell == FILLED), None)
            if first is not None:
                shift, tail = decode([EMPTY] * first + [FILLED]), decode(parts[first:])
                certificate(shift, 'mul', tail, residual)
                run(['./run_cmds.sh', 'tfactor read ' + tail])
            equation = f'{SOURCE} = {sign}{left} * {right} - ({residual_sign}{residual})'
            log.write('SIGNED SOURCE RETURN ' + equation + '\n')
            log.write(f'RESIDUAL SHAPE width={width} initial-empty-cells={first} total-cells={len(parts)}\n')
            summaries.append(f'Frame {width}, root `{root}`: initial empty correction cells {first}; correction cells {len(parts)}.\n\n`{equation}`\n')
            print(f'frame={width} residual-empty-cells={first} residual-cells={len(parts)} source-equation=PASS', flush=True)
            if residual == '0' and sign == '+' and left not in {'0', '1', SOURCE}:
                run(['./godel', 'product', SOURCE, left, right])
                print('FACTORS', left, right, flush=True)
        (DEST / 'frame_sum_returns.md').write_text(
            '# Full-frame sum returns\n\n'
            'The retained roots are read from the completed repaired-sum archive. '
            'Gödel certifies every signed source equation and the positioned correction decomposition. '
            'The native unit control passes; the altered-source control fails.\n\n' + '\n'.join(summaries))


if __name__ == '__main__':
    main()
