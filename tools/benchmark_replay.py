"""Compare JSON and lossless binary replay latency with complete output checks.

Worker parse, simulation, capture, encoding and pipe transfer are timed.
Caller request encoding and response decoding are outside the timed interval.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import statistics
import subprocess
import time
from replay_protocol import read_wire, decode

def measure(engines, wires, repeats, warmup=5, validation=True):
    workers = {name: subprocess.Popen([str(path.resolve())], stdin=subprocess.PIPE,
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
        env=dict(os.environ, GLOBALSIM_OUTPUT='binary-v1' if 'binary' in name else 'json',
                 GLOBALSIM_REPLAY_WORKER='1' if name.endswith('helper') else '0'))
        for name, path in engines.items()}
    elapsed = {name: [] for name in engines}
    sizes = {name: [] for name in engines}
    hashes = {name: [] for name in engines}
    # Cache validation by input and reply identity; don't repeatedly decode megabytes.
    checked = set()
    try:
        for index in range(warmup + repeats * len(wires)):
            wire = wires[index % len(wires)]
            order = list(workers.items())
            # Rotate, then reverse every other round to balance runs of >2 engines.
            rotate = index % len(order)
            order = order[rotate:] + order[:rotate]
            if index % 2: order.reverse()
            replies = {}
            for name, worker in order:
                start = time.perf_counter_ns()
                worker.stdin.write(wire)
                worker.stdin.flush()
                reply = read_wire(worker.stdout) if 'binary' in name else worker.stdout.readline()
                duration = (time.perf_counter_ns() - start) / 1e6
                assert reply, (name, worker.poll())
                replies[name] = reply
                if index >= warmup:
                    elapsed[name].append(duration)
                    sizes[name].append(len(reply))
                    hashes[name].append(hashlib.sha256(reply).hexdigest())
            signatures = tuple(hashlib.sha256(replies[n]).digest() for n in engines)
            if signatures not in checked and validation:
                reference = json.loads(replies['before'])
                assert 'error' not in reference, reference
                for name, reply in replies.items():
                    if 'binary' in name: decode(reply).assert_matches(reference)
                    else: assert reply == replies['before'], name
                checked.add(signatures)
        for w in workers.values():
            w.stdin.close()
            assert w.wait(timeout=30) == 0
    finally:
        for w in workers.values():
            if w.poll() is None: w.kill(); w.wait()
            w.stdout.close()
            if not w.stdin.closed: w.stdin.close()
    result = {name: {'runs': len(times), 'mean_ms': statistics.mean(times),
        'median_ms': statistics.median(times), 'p95_ms': sorted(times)[int(len(times)*.95)],
        'mean_bytes': statistics.mean(sizes[name]),
        'reply_sequence_sha256': hashlib.sha256(''.join(hashes[name]).encode()).hexdigest()}
        for name, times in elapsed.items()}
    result['compared_distinct_replies'] = len(checked)
    return result


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--engine', type=Path, required=True)
    p.add_argument('--baseline', type=Path)
    p.add_argument('--inputs', type=Path, required=True)
    p.add_argument('--report', type=Path)
    p.add_argument('--repeats', type=int, default=5)
    p.add_argument('--warmup', type=int, default=5)
    p.add_argument('--helper', action='store_true')
    p.add_argument('--standard-frames', action='store_true')
    args = p.parse_args()
    if args.repeats < 1 or args.warmup < 0:
        p.error('repeats must be positive and warmup nonnegative')
    specs = [json.loads(line) for line in args.inputs.read_text(encoding='utf-8-sig').splitlines() if line.strip()]
    if not specs: p.error('inputs must contain race requests')
    wires = [(json.dumps(dict(s, retainAllFrames=not args.standard_frames), separators=(',', ':'))+'\n').encode() for s in specs]
    engines = {'before': args.baseline or args.engine, 'json': args.engine, 'binary': args.engine}
    if args.helper: engines['json-helper'] = args.engine
    report = {
        'scope': 'Serial persistent workers in alternating order. Caller encode/decode excluded; complete frame/value comparisons outside timing.',
        'all_frames': not args.standard_frames, 'repeats': args.repeats, 'warmups': args.warmup,
        'engine_sha256': {n: hashlib.sha256(path.read_bytes()).hexdigest() for n,path in engines.items()},
        'inputs_sha256': hashlib.sha256(b''.join(wires)).hexdigest(),
        'timings': measure(engines, wires, args.repeats, args.warmup),
    }
    text = json.dumps(report, indent=2)+'\n'
    if args.report: args.report.write_text(text, encoding='utf-8')
    print(text, end='')


if __name__ == '__main__': main()
