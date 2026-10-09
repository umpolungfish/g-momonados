"""Certify a source-derived square correction by positioned one-bit lifts."""
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
EMPTY, FILLED = '≻⋈∈⊤∋', '≻⋈∈⊥∋'
SOURCE = sys.argv[1] if len(sys.argv) > 1 else '8051'
SINGLE_PRESENCE = '--single-presence' in sys.argv[2:]

def field(out, label):
    return next(line.split(None, 1)[1] for line in out.splitlines()
                if line.startswith(label + ' '))

def cells(word):
    return [word[1:-3][i:i + 5] for i in range(0, len(word[1:-3]), 5)]

with (ROOT / 'measurements' / ('godel_hex_square_bit_lift_' + SOURCE + '.log')).open('a') as record:
    def run(args, allow_underflow=False):
        out = subprocess.run(args, cwd=ROOT, text=True,
                             stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        record.write('COMMAND ' + repr(args) + '\n' + out.stdout +
                     '\nEXIT ' + str(out.returncode) + '\n\n')
        if out.returncode:
            if allow_underflow and 'frame subtraction underflow' in out.stdout:
                return None
            raise RuntimeError(out.stdout)
        return out.stdout

    def decode(parts):
        return field(run(['./godel', 'decode', '⊢' + ''.join(parts) + '⊙⊡⊣']), 'value')

    def encode(value):
        return cells(field(run(['./godel', 'encode', value]), 'word'))

    def read(value):
        out = run(['./run_cmds.sh', 'trilattice_factor read ' + value])
        clean = re.sub(r'\x1b\[[0-9;]*m', '', out)
        return cells(next(line.split(':', 1)[1].strip()
                          for line in clean.splitlines()
                          if 'native word      :' in line))

    def operate(a, op, b):
        left, right = encode(a), encode(b)
        width = max(2, len(left), len(right))
        composed = decode(left + [EMPTY] * (2 * width - len(left)) + right)
        return run(['./godel', 'frame-op', composed, str(width), '0', op,
                    str(2 * width), '1'], allow_underflow=(op == 'sub'))

    def product(n, a, b):
        out = run(['./godel', 'product', n, a, b])
        assert 'relation.exact-product     PASS' in out

    proper_certificates = []

    def source_product(a, b, label):
        product(SOURCE, a, b)
        proper = a not in {'0', '1', SOURCE} and b not in {'0', '1', SOURCE}
        record.write('SOURCE PRODUCT CERTIFICATE label=' + label + ' left=' + a +
                     ' right=' + b + ' exact=PASS proper=' + str(proper) + '\n')
        proper_certificates.append(proper)
        print(SOURCE, label, a, b, 'proper-factor PASS' if proper else 'unit return')
        return proper

    def shared_pair_return(value):
        unbraided = run(['./godel', 'unbraid', value])
        gamma = next(line[len('Γ lane'):].strip() for line in unbraided.splitlines()
                     if line.startswith('Γ lane '))
        lam = next(line[len('Λ lane'):].strip() for line in unbraided.splitlines()
                   if line.startswith('Λ lane '))
        if gamma != lam or gamma == '0':
            record.write('SHARED PAIR RETAINED: unequal positioned lanes\n')
            return None
        lane = cells(field(unbraided, 'Γ.word'))
        positioned = decode([part for cell in lane for part in (cell, EMPTY)])
        payload_pair = decode([FILLED, FILLED])
        product(value, payload_pair, positioned)
        read(positioned)
        record.write('SHARED PAIR RETURN ' + value + ' = ' + payload_pair +
                     ' * ' + positioned + ' lane=' + gamma + '\n')
        return payload_pair, positioned

    source = SOURCE
    source_parts = read(source)
    correction_parts = source_parts[4:8]
    correction_parts += [EMPTY] * (4 - len(correction_parts))
    low_source = decode(source_parts[:4])
    if low_source in {'1', '9'}:
        # Move the source's sole falsity presence into information.
        if correction_parts[1] == FILLED and correction_parts[2] == EMPTY:
            correction_parts[1], correction_parts[2] = correction_parts[2], correction_parts[1]
        if correction_parts[0] == FILLED and correction_parts[2] == EMPTY:
            correction_parts[0], correction_parts[2] = correction_parts[2], correction_parts[0]
    elif low_source in {'7', '11'}:
        # Retain information and its banked fork; remove falsity when equal.
        if correction_parts[1] == correction_parts[2] == FILLED:
            correction_parts[1] = EMPTY
    elif low_source in {'5', '13'}:
        if correction_parts[0] == FILLED and correction_parts[1] == EMPTY:
            correction_parts = [EMPTY] + correction_parts
    if SINGLE_PRESENCE and FILLED in correction_parts:
        first = correction_parts.index(FILLED)
        correction_parts = [EMPTY] * first + [FILLED]
        record.write('SECOND-MOTIF SINGLE-PRESENCE RETURN retained-slot=' + str(first) + '\n')
    payload = decode(correction_parts)
    read(payload)
    if payload == '0':
        record.write('RETAINED EMPTY SECOND HEX PAYLOAD\n')
        print(source, 'retained empty second hex payload')
        sys.exit(0)
    square = field(operate(payload, 'mul', payload), 'result')
    correction = field(operate(source, 'add', square), 'result')
    parts = read(correction)
    low_parts = parts[:4]
    low_parts += [EMPTY] * (4 - len(low_parts))
    low = decode(low_parts)
    if low not in {'0', '1', '4', '9'} and low_source in {'3', '15'} and FILLED in correction_parts:
        first_occupied = correction_parts.index(FILLED)
        record.write('LOW HEX CONSTRAINT SELECTS FIRST OCCUPIED SECOND-MOTIF SLOT ' +
                     str(first_occupied) + ' -> truth; retain one occupied unit\n')
        run(['python3', 'measurements/godel_hex_ordered_word_probe.py', source])
        correction_parts = [FILLED]
        payload = decode(correction_parts)
        read(payload)
        square = field(operate(payload, 'mul', payload), 'result')
        correction = field(operate(source, 'add', square), 'result')
        parts = read(correction)
        low_parts = parts[:4]
        low_parts += [EMPTY] * (4 - len(low_parts))
        low = decode(low_parts)
    if low not in {'0', '1', '4', '9'}:
        record.write('SQUARE RETURN REFUTED: corrected low hex value ' + low +
                     ' lies outside the verified square image {0,1,4,9}\n')
        print(source, 'square return refuted for this correction; low hex', low)
        sys.exit(0)
    initial_empty = 0
    for part in parts:
        if part != EMPTY:
            break
        initial_empty += 1
    shift = decode([EMPTY] * initial_empty + [FILLED])
    tail = decode(parts[initial_empty:])
    product(correction, shift, tail)
    target = encode(tail)
    target += [EMPTY] * max(0, 3 - len(target))
    if initial_empty % 2 or target[:3] != [FILLED, EMPTY, EMPTY]:
        record.write('SQUARE RETURN REFUTED BY EMPTY-RUN PARITY OR ODD LOW THREE CELLS\n')
        print(source, 'correction', payload, 'retained prefix refutation')
        sys.exit(0)
    root_parts = [FILLED]
    root_tail = None
    bound = (len(encode(tail)) + 1) // 2 + 1
    for position in range(3, max(4, bound + 1)):
        current = decode(root_parts)
        current_square = field(operate(current, 'mul', current), 'result')
        square_parts = encode(current_square)
        square_parts += [EMPTY] * max(0, position + 1 - len(square_parts))
        target += [EMPTY] * max(0, position + 1 - len(target))
        if square_parts[position] != target[position]:
            root_parts += [EMPTY] * (position - len(root_parts))
            assert root_parts[position - 1] == EMPTY
            root_parts[position - 1] = FILLED
            current = decode(root_parts)
            current_square = field(operate(current, 'mul', current), 'result')
            square_parts = encode(current_square)
            square_parts += [EMPTY] * max(0, position + 1 - len(square_parts))
        assert square_parts[:position + 1] == target[:position + 1]
        modulus = decode([EMPTY] * position + [FILLED])
        opposite = field(operate(modulus, 'sub', current), 'result')
        opposite_square = field(operate(opposite, 'mul', opposite), 'result')
        record.write('BIT LIFT position=' + str(position) + ' root=' + current +
                     ' opposite=' + opposite + ' prefix=closed\n')
        if current_square == tail:
            root_tail = current
            break
        if opposite_square == tail:
            root_tail = opposite
            break
    if root_tail is None:
        for value in (current, opposite, current_square, opposite_square):
            read(value)
        root_shift = decode([EMPTY] * (initial_empty // 2) + [FILLED])
        coefficients = []
        for returned in (current, opposite):
            restored = field(operate(root_shift, 'mul', returned), 'result')
            original_left = operate(restored, 'sub', payload)
            original_left_sign = '+'
            if original_left is None:
                original_left = operate(payload, 'sub', restored)
                original_left_sign = '-'
            left = field(original_left, 'result')
            right = field(operate(restored, 'add', payload), 'result')
            positioned_product = field(operate(left, 'mul', right), 'result')
            if original_left_sign == '-':
                difference = operate(source, 'add', positioned_product)
                sign = '-'
            else:
                difference = operate(positioned_product, 'sub', source)
                sign = '+'
                if difference is None:
                    difference = operate(source, 'sub', positioned_product)
                    sign = '-'
            residual = field(difference, 'result')
            read(residual)
            residual_parts = encode(residual)
            residual_zeros = 0
            for cell in residual_parts:
                if cell != EMPTY:
                    break
                residual_zeros += 1
            placement = decode([EMPTY] * (position + 1 + initial_empty) + [FILLED])
            coefficient = operate(residual, 'divmod', placement)
            assert field(coefficient, 'remainder') == '0'
            quotient = field(coefficient, 'result')
            coefficients.append((sign, quotient))
            product(residual, placement, quotient)
            read(quotient)
            record.write('SIGNED SOURCE RETURN ' + source + ' = ' + original_left_sign + left +
                         ' * ' + right + ' - (' + sign + residual + ')\n')
            record.write('PLACED RESIDUAL ' + sign + residual + ' = ' +
                         sign + quotient + ' * ' + placement +
                         ' initial-empty-cells=' + str(residual_zeros) + '\n')
            correction_zeros = 0
            for cell in encode(payload):
                if cell != EMPTY:
                    break
                correction_zeros += 1
            root_zeros = initial_empty // 2
            if residual_zeros < 2 * max(root_zeros, correction_zeros) + 3:
                record.write('CO-LIFT RETAINED: residual cell below simultaneous-toggle bound\n')
            root_increment = decode([EMPTY] * (residual_zeros - root_zeros - 1) + [FILLED])
            correction_increment = decode([EMPTY] * (residual_zeros - correction_zeros - 1) + [FILLED])
            changed_root = field(operate(restored, 'add', root_increment), 'result')
            changed_correction = field(operate(payload, 'add', correction_increment), 'result')
            for label, lifted_root, lifted_correction in (
                    ('root', changed_root, payload),
                    ('correction', restored, changed_correction)):
                applicable_valuation = root_zeros if label == 'root' else correction_zeros
                if residual_zeros < 2 * applicable_valuation + 3:
                    record.write('INDIVIDUAL LIFT RETAINED ' + label +
                                 ': residual cell below its valuation bound\n')
                    print(source, label, 'retained below individual lift bound')
                    continue
                lifted_square = field(operate(lifted_root, 'mul', lifted_root), 'result')
                lifted_correction_square = field(operate(lifted_correction, 'mul', lifted_correction), 'result')
                target_square = field(operate(source, 'add', lifted_correction_square), 'result')
                transported = operate(lifted_square, 'sub', target_square)
                transported_sign = '+'
                if transported is None:
                    transported = operate(target_square, 'sub', lifted_square)
                    transported_sign = '-'
                transported_residual = field(transported, 'result')
                transported_parts = encode(transported_residual)
                transported_zeros = 0
                for cell in transported_parts:
                    if cell != EMPTY:
                        break
                    transported_zeros += 1
                assert transported_residual == '0' or transported_zeros > residual_zeros
                for value in (lifted_root, lifted_correction, transported_residual):
                    read(value)
                record.write('CORRELATED LIFT ' + label + ' H=' + lifted_root +
                             ' a=' + lifted_correction + ' signed-residual=' +
                             transported_sign + transported_residual +
                             ' initial-empty-cells=' + str(transported_zeros) + '\n')
                left_return = operate(lifted_root, 'sub', lifted_correction)
                left_sign = '+'
                if left_return is None:
                    left_return = operate(lifted_correction, 'sub', lifted_root)
                    left_sign = '-'
                left_magnitude = field(left_return, 'result')
                right_return = field(operate(lifted_root, 'add', lifted_correction), 'result')
                for value in (left_magnitude, right_return):
                    read(value)
                returned_product = field(operate(left_magnitude, 'mul', right_return), 'result')
                reconstructed = operate(source, 'add' if transported_sign == '+' else 'sub',
                                        transported_residual)
                if reconstructed is None:
                    assert left_sign == '-'
                    reconstructed = operate(transported_residual, 'sub', source)
                else:
                    assert left_sign == '+'
                assert field(reconstructed, 'result') == returned_product
                record.write('SIGNED OPERAND RETURN p=' + left_sign + left_magnitude +
                             ' q=+' + right_return + ' exact-source-residual=PASS\n')
                if left_sign == '+' and transported_residual != '0':
                    for operand, other in ((left_magnitude, right_return),
                                           (right_return, left_magnitude)):
                        division = operate(transported_residual, 'divmod', operand)
                        collected_quotient = field(division, 'result')
                        collected_remainder = field(division, 'remainder')
                        record.write('LIFTED RESIDUAL COLLECTION ' + label +
                                     ' operand=' + operand + ' quotient=' +
                                     collected_quotient + ' remainder=' +
                                     collected_remainder + '\n')
                        cofactor = field(operate(other, 'sub' if transported_sign == '+' else 'add',
                                                 collected_quotient), 'result')
                        partial_product = field(operate(operand, 'mul', cofactor), 'result')
                        if collected_remainder == '0':
                            recovered = partial_product
                        else:
                            recovered = field(operate(partial_product,
                                                      'sub' if transported_sign == '+' else 'add',
                                                      collected_remainder), 'result')
                        assert recovered == source
                        read(cofactor)
                        record.write('PARTIAL SOURCE COLLECTION n=' + source +
                                     ' operand=' + operand + ' cofactor=' + cofactor +
                                     ' residual-sign=' + transported_sign +
                                     ' remainder=' + collected_remainder + ' EXACT PASS\n')
                        if collected_remainder == '0':
                            source_product(operand, cofactor, 'lifted residual collection')
                        else:
                            read(collected_remainder)
                            paired = shared_pair_return(collected_remainder)
                            if paired is not None and paired[0] == cofactor:
                                coupled_quotient, coupled_remainder = paired[1], '0'
                                record.write('COFACTOR COLLECTS THROUGH SHARED HEX/LANE PAYLOAD\n')
                            else:
                                coupled_return = operate(collected_remainder, 'divmod', cofactor)
                                coupled_quotient = field(coupled_return, 'result')
                                coupled_remainder = field(coupled_return, 'remainder')
                            record.write('PARTIAL REMAINDER/COFACTOR cofactor=' + cofactor +
                                         ' quotient=' + coupled_quotient +
                                         ' remainder=' + coupled_remainder + '\n')
                            if coupled_remainder == '0':
                                collected_operand = field(operate(operand,
                                    'sub' if transported_sign == '+' else 'add',
                                    coupled_quotient), 'result')
                                source_product(cofactor, collected_operand, 'partial cofactor collection')
                    print(source, label, 'positive operand collection checked')
                print(source, label, 'operand signs', left_sign + '+')
                if transported_residual == '0':
                    factor_left = field(operate(lifted_root, 'sub', lifted_correction), 'result')
                    factor_right = field(operate(lifted_root, 'add', lifted_correction), 'result')
                    source_product(factor_left, factor_right, 'correlated collection')
                print(source, label, 'lift repairs source bit', residual_zeros,
                      'new initial empty cells', transported_zeros)
            for candidate, counterpart in (((left, right), (right, left))
                                           if original_left_sign == '+' else ()):
                collected = operate(residual, 'divmod', candidate)
                rem = field(collected, 'remainder')
                record.write('RESIDUAL/PAYLOAD ' + candidate + ' remainder=' + rem + '\n')
                if rem == '0':
                    cofactor = field(operate(counterpart, 'sub' if sign == '+' else 'add',
                                             field(collected, 'result')), 'result')
                    source_product(candidate, cofactor, 'residual collection')
            print(source, 'signed residual coefficient', sign + quotient,
                  'initial empty cells', residual_zeros)
        if coefficients[0][0] == '-' and coefficients[1][0] == '+':
            delta = field(operate(coefficients[0][1], 'add', coefficients[1][1]), 'result')
            half_modulus = decode([EMPTY] * (position - 1) + [FILLED])
            predicted = field(operate(half_modulus, 'sub', current), 'result')
            assert delta == predicted
            record.write('PAIRED COEFFICIENT TRANSPORT C2-C1 = 2^(k-1)-h = ' + delta +
                         ' EXACT PASS\n')
        record.write('FINITE ROOT WIDTH REACHED WITHOUT EXACT SQUARE RETURN\n')
        print(source, 'correction', payload, 'bit-lift prefixes closed through',
              position, 'retained final roots', current, opposite)
        if not SINGLE_PRESENCE and not any(proper_certificates) and low_source in {'5', '13'} and correction_parts.count(FILLED) > 1:
            record.write('RETAIN FULL RETURN; FOLLOW FIRST OCCUPIED SECOND-MOTIF PRESENCE ONCE\n')
            record.flush()
            out = run(['python3', 'measurements/godel_hex_square_bit_lift_probe.py', source, '--single-presence'])
            print(out.rstrip())
        sys.exit(0)
    product(tail, root_tail, root_tail)
    root_shift = decode([EMPTY] * (initial_empty // 2) + [FILLED])
    product(shift, root_shift, root_shift)
    root = field(operate(root_shift, 'mul', root_tail), 'result')
    product(correction, root, root)
    left = field(operate(root, 'sub', payload), 'result')
    right = field(operate(root, 'add', payload), 'result')
    proper = source_product(left, right, 'square collection')
    for value in (payload, square, root, left, right):
        read(value)
    record.write(('SOURCE-ONLY SQUARE COLLECTION ' if proper else 'UNIT SOURCE RETURN ') + source + ' = ' + left +
                 ' * ' + right + '\n')
    print(source, '+', square, '=', root, '*', root, '; factors', left, right,
          'PASS')
