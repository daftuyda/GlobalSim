"""Measure a pool of independent persistent workers, retaining every race frame.

Batch wall time per race measures throughput; individual request latency is
reported separately. Client pipe handling and reply hashing are included in
batch wall time. Worker startup and warmup are excluded.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import statistics
import subprocess
import threading
import time
from replay_protocol import read_wire


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--engine', type=Path, required=True)
    p.add_argument('--inputs', type=Path, required=True)
    p.add_argument('--workers', type=int, nargs='+', default=[1, 2, 4, 8])
    p.add_argument('--repeats', type=int, default=3)
    p.add_argument('--requests', type=int, default=240)
    p.add_argument('--format', choices=['json', 'binary-v1'], default='binary-v1')
    p.add_argument('--report', type=Path)
    args = p.parse_args()
    if args.repeats < 1 or args.requests < 1 or any(n < 1 for n in args.workers):
        p.error('counts must be positive')
    specs = [json.loads(line) for line in args.inputs.read_text(encoding='utf-8-sig').splitlines() if line.strip()]
    if not specs: p.error('inputs must contain races')
    wires = [(json.dumps(dict(s, retainAllFrames=True), separators=(',', ':'))+'\n').encode() for s in specs]
    env = dict(os.environ, GLOBALSIM_OUTPUT=args.format, GLOBALSIM_REPLAY_WORKER='0')
    def start():
        return subprocess.Popen([str(args.engine.resolve())], stdin=subprocess.PIPE,
            stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, env=env)
    def request(w, wire):
        before = time.perf_counter_ns()
        w.stdin.write(wire); w.stdin.flush()
        reply = read_wire(w.stdout) if args.format == 'binary-v1' else w.stdout.readline()
        elapsed = (time.perf_counter_ns() - before) / 1e6
        if not reply: raise RuntimeError('Worker exited without a reply')
        return elapsed, hashlib.sha256(reply).digest()
    def close(w):
        if not w.stdin.closed: w.stdin.close()
        try: w.wait(timeout=30)
        except subprocess.TimeoutExpired: w.kill(); w.wait()
        w.stdout.close()
        if w.returncode: raise RuntimeError(f'Worker exited with {w.returncode}')
    reference_worker = start()
    try: expected = [request(reference_worker, wire)[1] for wire in wires]
    finally: close(reference_worker)
    report = {'scope': __doc__, 'format': args.format,
        'engine_sha256': hashlib.sha256(args.engine.read_bytes()).hexdigest(),
        'inputs_sha256': hashlib.sha256(b''.join(wires)).hexdigest(),
        'requests_per_trial': args.requests, 'trials': []}
    for repeat in range(args.repeats):
        order = args.workers if repeat % 2 == 0 else args.workers[::-1]
        for count in order:
            workers = [start() for _ in range(count)]
            gate = threading.Barrier(count + 1)
            ready = threading.Barrier(count + 1)
            def serve(i, worker):
                for j in range(5):
                    _, digest = request(worker, wires[j % len(wires)])
                    assert digest == expected[j % len(wires)]
                ready.wait(timeout=120)
                gate.wait(timeout=120)
                times = []
                for j in range(i, args.requests, count):
                    elapsed, digest = request(worker, wires[j % len(wires)])
                    assert digest == expected[j % len(wires)], 'Output changed under concurrency'
                    times.append(elapsed)
                return times
            try:
                with ThreadPoolExecutor(count) as pool:
                    futures = [pool.submit(serve, i, w) for i, w in enumerate(workers)]
                    ready.wait(timeout=120)
                    before = time.perf_counter_ns()
                    gate.wait(timeout=120)
                    times = [x for future in futures for x in future.result()]
                    elapsed = (time.perf_counter_ns() - before) / 1e6
                trial = {'repeat': repeat, 'workers': count, 'requests': len(times),
                    'batch_ms': elapsed, 'wall_ms_per_race': elapsed / len(times),
                    'races_per_second': len(times) * 1000 / elapsed,
                    'individual_request_mean_ms': statistics.mean(times),
                    'individual_request_p95_ms': sorted(times)[int(len(times)*.95)]}
                report['trials'].append(trial)
                print(json.dumps(trial), flush=True)
            finally:
                for w in workers: close(w)
    if args.report: args.report.write_text(json.dumps(report, indent=2)+'\n', encoding='utf-8')


if __name__ == '__main__': main()
