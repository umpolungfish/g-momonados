#!/usr/bin/env python3
"""Start and inspect the local Fibonacci braid socket without replacing files."""
import hashlib
import errno
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
OUT = HERE
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
        mode = SOCKET.lstat().st_mode
        if not stat.S_ISSOCK(mode) or SOCKET.lstat().st_uid != os.getuid():
            raise RuntimeError(f'preserving existing non-socket: {SOCKET}')
        try:
            with connect() as stream:
                reader = stream.makefile('r')
                writer = stream.makefile('w')
                status = request(writer, reader, {'op': 'status'})
                if status.get('backend') != 'contracted_fibonacci_ququart':
                    raise RuntimeError(f'preserving a different socket service: {status}')
        except ConnectionRefusedError:
            old_pid = int(PID.read_text()) if PID.exists() else None
            if old_pid is not None:
                try:
                    os.kill(old_pid, 0)
                except ProcessLookupError:
                    pass
                else:
                    raise RuntimeError(f'prior socket process {old_pid} is still running')
            SOCKET.unlink()
    if not SOCKET.exists():
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
        if status.get('backend') != 'contracted_fibonacci_ququart':
            raise RuntimeError(f'factor backend is unavailable: {status}')
        assert status['ok'] and status['factor_execution_available'] is True
    report = {'socket': str(SOCKET), 'pid': int(PID.read_text()) if PID.exists() else None,
              'binary_sha256': hashlib.sha256(BINARY.read_bytes()).hexdigest(),
              'status': status}
    REPORT.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'socket': str(SOCKET), 'pid': report['pid'],
                      'status': status}))


if __name__ == '__main__':
    main()
