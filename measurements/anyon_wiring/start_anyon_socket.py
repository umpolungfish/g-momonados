#!/usr/bin/env python3
"""Start and inspect the local Fibonacci braid socket without replacing files."""
import hashlib
import json
import os
from pathlib import Path
import socket
import stat
import subprocess
import time

HERE = Path(__file__).resolve().parent
PROJECT = HERE.parents[1]
BINARY = HERE / 'anyon_socket_server'
SOCKET = Path(os.environ.get('G_MOMONADOS_ANYON_SOCKET',
                             '/tmp/g-momonados-fibonacci.sock'))
OUT = HERE / 'socket_braids'
LOG = HERE / 'anyon_socket.log'
PID = HERE / 'anyon_socket.pid'
REPORT = HERE / 'anyon_socket.json'


def connect():
    stream = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    stream.settimeout(5)
    try:
        stream.connect(str(SOCKET))
    except BaseException:
        stream.close()
        raise
    return stream


def request(writer, reader, value):
    writer.write(json.dumps(value) + '\n')
    writer.flush()
    line = reader.readline()
    if not line:
        raise RuntimeError('socket closed before replying')
    return json.loads(line)


def main():
    child = None
    if SOCKET.exists() or SOCKET.is_symlink():
        if not stat.S_ISSOCK(SOCKET.lstat().st_mode):
            raise RuntimeError(f'preserving existing non-socket: {SOCKET}')
    else:
        OUT.mkdir(parents=True, exist_ok=True)
        with LOG.open('ab') as log:
            child = subprocess.Popen([str(BINARY), str(SOCKET), str(OUT)],
                                     stdin=subprocess.DEVNULL,
                                     stdout=log, stderr=log,
                                     start_new_session=True)
        PID.write_text(str(child.pid) + '\n')
        deadline = time.monotonic() + 5
        while not SOCKET.exists():
            if child.poll() is not None:
                raise RuntimeError(f'server exited {child.returncode}; see {LOG}')
            if time.monotonic() >= deadline:
                raise RuntimeError(f'socket startup timed out; see {LOG}')
            time.sleep(0.05)

    with connect() as stream:
        reader = stream.makefile('r')
        writer = stream.makefile('w')
        status = request(writer, reader, {'op': 'status'})
        if status.get('backend') != 'braid_compiler':
            raise RuntimeError(f'preserving a different socket service: {status}')
        assert status['ok'] and status['fusion_readout_available'] is False
        records = []
        cases = json.loads((PROJECT.parent / 'Vox/measurements/anyon_wiring/rsa_cases.json').read_text())
        for shot_id, case in enumerate(cases, 1):
            n = int(case['n'])
            width = n.bit_length()
            assert width >= 200
            phase_bits = 2 * width + 8
            begin = request(writer, reader, {
                'op': 'begin', 'protocol': status['protocol'],
                'shot_id': shot_id, 'format': 'little_endian_bits',
                'source': bin(n)[2:][::-1], 'base': '01',
                'logical_qubits': 3 * width + 4, 'phase_bits': phase_bits,
                'work_preparation': 'uniform_residues',
            })
            assert begin == {'ok': True, 'shot_id': shot_id}
            for generator in [1, -1]:
                writer.write(json.dumps({'op': 'exchange', 'shot_id': shot_id,
                                         'generator': generator}) + '\n')
            replies = []
            for index in [0, phase_bits - 1]:
                reply = request(writer, reader, {
                    'op': 'measure_control_fusion', 'shot_id': shot_id,
                    'phase_index': index,
                })
                assert reply['shot_id'] == shot_id and reply['phase_index'] == index
                assert reply['error'] == 'fusion execution backend is not configured'
                assert 'fusion_bit' not in reply
                saved = Path(reply['compiled_braid']).read_text().splitlines()
                assert saved[1:] == ['1', '-1']
                assert bin(n)[2:][::-1] in saved[0]
                replies.append(reply)
            end = request(writer, reader, {'op': 'abort', 'shot_id': shot_id})
            assert end == {'ok': True, 'shot_id': shot_id}
            records.append({'bits': width, 'source': str(n),
                            'connection_verified': True, 'readout_checks': replies})
    report = {'socket': str(SOCKET), 'pid': int(PID.read_text()) if PID.exists() else None,
              'binary_sha256': hashlib.sha256(BINARY.read_bytes()).hexdigest(),
              'status': status, 'controls': records}
    REPORT.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'socket': str(SOCKET), 'pid': report['pid'],
                      'status': status, 'verified_source_bits': [r['bits'] for r in records]}))


if __name__ == '__main__':
    main()
