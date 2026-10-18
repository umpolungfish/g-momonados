#!/usr/bin/env python3
"""Check retained payload bytes inside the compiled static ELF without running it."""
import argparse
import gzip
import hashlib
import json
import mmap
from pathlib import Path
import struct


def symbol_ranges(image, names):
    assert image[:6] == b'\x7fELF\x02\x01', 'expected a little-endian ELF64 image'
    header = struct.unpack_from('<16sHHIQQQIHHHHHH', image)
    sections = [struct.unpack_from('<IIQQQQIIQQ', image, header[6] + i * header[11])
                for i in range(header[12])]
    result = {}
    for section in sections:
        if section[1] != 2:
            continue
        strings = sections[section[6]]
        for pos in range(section[4], section[4] + section[5], section[9]):
            name, _, _, index, address, size = struct.unpack_from('<IBBHQQ', image, pos)
            start = strings[4] + name
            label = image[start:image.find(b'\0', start)].decode()
            if label not in names:
                continue
            owner = sections[index]
            assert owner[2] & 2 and not owner[2] & 1, 'payload is not read-only allocated data'
            offset = owner[4] + address - owner[3]
            assert owner[4] <= offset <= offset + size <= owner[4] + owner[5]
            assert label not in result, 'duplicate payload symbol'
            result[label] = (offset, size)
    assert set(result) == set(names), 'missing retained payload symbol'
    return result


def verify(binary, project):
    compilation = json.loads((project / 'compilation.json').read_text())
    for path in [binary, project / 'parent_return.hex', project / 'deferred_source_arms.cells',
                 project / 'retained_payload.S', project / 'retained_payload.o']:
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        assert compilation['bindings'][str(path.resolve())] == digest, 'compilation binding changed'
    parent_blob = b''
    members = []
    if compilation['resumed_from_sha256']:
        parent_blob = (project / 'parent_return.json.gz').read_bytes()
        parent_raw = gzip.decompress(parent_blob)
        assert hashlib.sha256(parent_raw).hexdigest() == compilation['resumed_from_sha256']
        parent = json.loads(parent_raw)
        members = sorted(parent['members'], key=lambda m: (m['dialect'], m['operation']))
    cells = bytes(cell for member in members for word in member.get('deferred_source_arms', []) for cell in word)
    assert all(cell in (8, 9) for cell in cells), 'invalid IMASM numeral cell'
    assert (project / 'deferred_source_arms.cells').read_bytes() == (cells or bytes([8]))
    assert (project / 'parent_return.hex').read_bytes() == parent_blob.hex().encode('ascii') + b'\0'
    payloads = {'PARENT_RETURN_GZIP_HEX': project / 'parent_return.hex',
                'RESUMED_DEFERRED_ARMS': project / 'deferred_source_arms.cells'}
    reports = {}
    with binary.open('rb') as executable, mmap.mmap(executable.fileno(), 0, access=mmap.ACCESS_READ) as image:
        ranges = symbol_ranges(image, payloads)
        for label, path in payloads.items():
            offset, size = ranges[label]
            assert size == path.stat().st_size, 'embedded payload length changed'
            digest = hashlib.sha256()
            with path.open('rb') as payload:
                checked = 0
                while chunk := payload.read(1024 * 1024):
                    assert image[offset + checked:offset + checked + len(chunk)] == chunk, 'embedded payload bytes changed'
                    digest.update(chunk)
                    checked += len(chunk)
            reports[label] = {'bytes_checked': checked, 'sha256': digest.hexdigest()}
    return {'format': 'dialect-static-payload-verification-v1', 'binary': str(binary),
            'parent_sha256': compilation['resumed_from_sha256'],
            'deferred_numeral_cells_checked': len(cells), 'payloads': reports}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary', type=Path)
    parser.add_argument('project', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    report = verify(args.binary.resolve(), args.project.resolve())
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))
