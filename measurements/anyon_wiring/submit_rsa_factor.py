#!/usr/bin/env python3
"""Submit the committed RSA semiprimes to the local factor socket."""
import json
import os
from pathlib import Path
import socket
import threading
import time

HERE = Path(__file__).resolve().parent
SOCKET = Path(os.environ.get('G_MOMONADOS_ANYON_SOCKET', '/tmp/g-momonados-fibonacci.sock'))
CASES = Path(__file__).resolve().parents[3] / 'Vox/measurements/anyon_wiring/rsa_cases.json'


def watch(done):
    while not done.wait(10):
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as connection:
            connection.settimeout(10)
            try:
                connection.connect(str(SOCKET))
                reader = connection.makefile('r')
                writer = connection.makefile('w')
                writer.write(json.dumps({'op':'status'}) + '\n')
                writer.flush()
                live = json.loads(reader.readline()).get('live', {})
                print('factor_live ' + json.dumps(live), flush=True)
            except OSError as error:
                print(f'factor_status_read: {error}', flush=True)


def main():
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as connection:
        connection.settimeout(3600)
        connection.connect(str(SOCKET))
        reader = connection.makefile('r')
        writer = connection.makefile('w')
        writer.write(json.dumps({'op':'status'}) + '\n')
        writer.flush()
        status = json.loads(reader.readline())
        assert status['factor_execution_available'] is True
        assert status['backend'] == 'contracted_fibonacci_ququart'
        done = threading.Event()
        monitor = threading.Thread(target=watch, args=(done,), daemon=True)
        monitor.start()
        cases = json.loads(CASES.read_text())
        for case in cases:
            source = int(case['n'])
            width = source.bit_length()
            expected = {int(case['p']), int(case['q'])}
            assert width >= 200 and int(case['p']) * int(case['q']) == source
            request = {'op':'factor', 'source':str(source), 'base':'2', 'max_shots':32}
            writer.write(json.dumps(request) + '\n')
            writer.flush()
            while True:
                line = reader.readline()
                if not line:
                    raise RuntimeError(f'{width}-bit factor service closed its connection')
                event = json.loads(line)
                if event.get('source') not in (None, str(source)):
                    raise RuntimeError(f'{width}-bit event was bound to another source')
                if event.get('event') == 'phase_measured':
                    completed = event['phase_digits_completed']
                    total = event['phase_digits_total']
                    if completed % 16 == 0 or completed == total:
                        print(f"{width}-bit shot {event['shot']}: phase {completed}/{total}", flush=True)
                elif event.get('event') == 'started':
                    print(f"{width}-bit source submitted to {event['backend']}", flush=True)
                elif event.get('event') == 'factored':
                    p, q = int(event['p']), int(event['q'])
                    assert event['product_closed'] is True and p*q == source
                    assert {p, q} == expected
                    print(f"{source} = {p} x {q}; order={event['order']}", flush=True)
                    done.set()
                    break
                elif event.get('event') == 'nonclosing_budget':
                    raise RuntimeError(f"{width}-bit source exhausted {event['max_shots']} shots")
                elif event.get('event') == 'error' or 'error' in event:
                    done.set()
                    raise RuntimeError(f"{width}-bit factor service: {event.get('error')}")


if __name__ == '__main__':
    main()
