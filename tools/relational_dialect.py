#!/usr/bin/env python3
"""Execute native dialect composition on bound ports of joint arithmetic rows.

The operation binding is explicit: two port objects compose into a third.
The resulting twelve type programs act on that output's current carrier.
Original arithmetic rows remain banked independently of the transformed state.
"""
from copy import deepcopy
from dataclasses import dataclass
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


class NativeRules:
    def __init__(self, table_path, execution_path):
        self.tables = json.loads(Path(table_path).read_text())
        self.execution = json.loads(Path(execution_path).read_text())
        for path, expected in self.execution['core_sources'].items():
            if digest(path) != expected:
                raise ValueError(f'native core changed: {path}')
        self.programs = {}
        for program in self.execution['programs']:
            if digest(program['source']) != program['sha256']:
                raise ValueError('native type program changed')
            self.programs[program['name']] = program
        source = (ROOT / 'G-mOMonadOS/src/catalog.rs').read_text()
        source = source.split('pub fn primitive_glyph(')[1].split('\npub fn ')[0]
        names = dict(re.findall(r'IgPrim::(\w+)\s*=>\s*"([^"]+)"', source))
        self.name_by_glyph = {glyph: name.rstrip('_') for name, glyph in names.items()}
        self.name_by_glyph['𐑣'] = 'ha'
        winding = (ROOT / 'G-mOMonadOS/src/prime_winding.rs').read_text()
        block = winding.split('const GROUNDED_HEX_DIGIT_TUPLES:')[1].split('\n};')[0]
        literals = re.findall(r'IgTuple\s*\{([^}]+)\}', block)
        if len(literals) != 16:
            raise ValueError('grounded digit table does not have sixteen entries')
        self.digit_types = []
        for literal in literals:
            fields = re.findall(r'\b(?:d|t|r|p|f|k|g|c|phi|h|s|omega):\s*(\w+)', literal)
            self.digit_types.append(tuple(next(i for i, value in enumerate(axis)
                                              if value['glyph'] == names[name])
                                          for axis, name in zip(self.tables['axis_values'], fields)))
        if any(len(v) != 12 for v in self.digit_types):
            raise ValueError('incomplete grounded digit type')
        self.bindings = {str(Path(table_path).resolve()): digest(table_path),
                         str(Path(execution_path).resolve()): digest(execution_path)}
        for p in ['G-mOMonadOS/src/prime_winding.rs', 'G-mOMonadOS/src/catalog.rs',
                  'G-mOMonadOS/src/algebra.rs', 'G-mOMonadOS/src/dialect_expansion.rs']:
            self.bindings[str(ROOT / p)] = digest(ROOT / p)
        self.bindings.update(self.execution['core_sources'])
        self.bindings.update({p['source']: p['sha256'] for p in self.execution['programs']})
        self.checked_native_compositions = 0
        cards = [len(axis) for axis in self.tables['axis_values']]
        def decode(address):
            result = []
            for card in reversed(cards):
                result.append(address % card)
                address //= card
            if address:
                raise ValueError('native tuple address exceeds its carrier')
            return tuple(reversed(result))
        def encode(coordinates):
            result = 0
            for card, coordinate in zip(cards, coordinates):
                result = result * card + coordinate
            return result
        for dialect in self.tables['tables']:
            for op, checks in zip(self.tables['operation_names'], dialect['full_tuple_checks']):
                for left, right, expected, conflict in checks:
                    actual, evidence = self.compose(dialect['id'], op, decode(left), decode(right))
                    if encode(actual) != expected or sum(int(v) << i for i, v in enumerate(evidence)) != conflict:
                        raise ValueError('axis action disagrees with native full-tuple execution')
                    self.checked_native_compositions += 1

    def compose(self, dialect, operation, left, right):
        if not 0 <= dialect < len(self.tables['tables']):
            raise ValueError('unknown native dialect')
        if len(left) != 12 or len(right) != 12:
            raise ValueError('native composition requires two complete tuples')
        rule = self.tables['tables'][dialect]
        op = self.tables['operation_names'].index(operation)
        result, conflicts = [], []
        for axis, values in enumerate(self.tables['axis_values']):
            cell = rule['axis_operations'][op][axis][left[axis] * len(values) + right[axis]]
            result.append(cell[0])
            conflicts.append(bool(cell[1]))
        return tuple(result), conflicts

    def run_types(self, types, seeds):
        # Positions belong to the type motifs. Do not fold their terminal
        # masks into one scalar and discard the twelve node/edge banks.
        if len(seeds) != 12:
            raise ValueError("type motifs require their twelve positioned edge seeds")
        trace, outputs = [], []
        for axis, index in enumerate(types):
            glyph = self.tables['axis_values'][axis][index]['glyph']
            program = self.programs[self.name_by_glyph[glyph]]
            seed = seeds[axis]
            executed = program['executions'][seed]
            if executed['seed'] != seed:
                raise ValueError('native transition seeds are not indexed exactly')
            trace.append({'axis': axis, 'program': program['name'],
                          'seed': seed, 'terminal': executed['terminal'],
                          'node_inputs': executed['node_inputs'], 'edge_values': executed['edge_values']})
            outputs.append(executed['terminal'])
        return outputs, trace


@dataclass
class TypedJointRelation:
    source_word: str
    ports: tuple
    origins: list
    rows: list
    types: list
    history: list

    def __post_init__(self):
        # JSON returns arrays; normalize their representation before measuring
        # semantic type changes across an executable continuation.
        self.ports = tuple(self.ports)
        self.types = [[tuple(port) for port in row] for row in self.types]

    @classmethod
    def arithmetic(cls, source_word, ports, rows, rules):
        if not source_word or len(set(ports)) != len(ports):
            raise ValueError('source or shared arithmetic port binding is invalid')
        if any(len(row) != len(ports) or any(v not in (1, 2) for v in row) for row in rows):
            raise ValueError('arithmetic rows must contain definite native truth/falsity cells')
        types = [[rules.digit_types[1 if value == 1 else 0] for value in row] for row in rows]
        return cls(source_word, tuple(ports), deepcopy(rows),
                   [[[value] * 12 for value in row] for row in rows], types, [])

    def compose_into(self, rules, dialect, operation, left_port, right_port, output_port):
        """Lift a bound binary morphism; operands are live, correlated port objects."""
        left, right, output = [self.ports.index(p) for p in (left_port, right_port, output_port)]
        result = deepcopy(self)
        observations = []
        for i, row in enumerate(self.rows):
            types, conflicts = rules.compose(dialect, operation, self.types[i][left], self.types[i][right])
            transformed, trace = rules.run_types(types, row[output])
            result.types[i][output] = types
            result.rows[i][output] = transformed
            observations.append({'row': i, 'left_type': self.types[i][left],
                                 'right_type': self.types[i][right], 'result_type': types,
                                 'conflicts': conflicts, 'before': row[output],
                                 'after': transformed, 'native_execution': trace})
        result.history.append({'dialect': dialect, 'operation': operation,
                               'composition_token': {'meet': '⋈', 'join': '∋'}.get(operation),
                               'ports': [left_port, right_port, output_port],
                               'rows': observations})
        return result
