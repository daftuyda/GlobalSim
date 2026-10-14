"""Compare every binary replay value with the conventional JSON interface."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
from check_worker_parity import isolation_cases
from replay_protocol import decode, read_wire


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--engine', type=Path, required=True)
    p.add_argument('--baseline', type=Path, required=True)
    p.add_argument('--inputs', type=Path, required=True)
    p.add_argument('--isolation', action='store_true')
    p.add_argument('--report', type=Path)
    args = p.parse_args()
    specs = [json.loads(line) for line in args.inputs.read_text(encoding='utf-8-sig').splitlines() if line.strip()]
    if not specs: p.error('inputs must contain races')
    cases = [(f'input-{i+1}', s) for i, s in enumerate(specs)]
    if args.isolation: cases.extend(isolation_cases(specs))
    report = {'engines': {n: hashlib.sha256(path.read_bytes()).hexdigest()
        for n, path in [('baseline', args.baseline), ('candidate', args.engine)]},
        'inputs_sha256': hashlib.sha256(args.inputs.read_bytes()).hexdigest(), 'modes': []}
    for dense in [False, True]:
        workers = {name: subprocess.Popen([str(path.resolve())], stdin=subprocess.PIPE,
            stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
            env=dict(os.environ, GLOBALSIM_OUTPUT='json' if name == 'baseline' else 'binary-v1',
                     GLOBALSIM_REPLAY_WORKER='1' if name == 'binary-helper' else '0'))
            for name, path in [('baseline', args.baseline), ('binary', args.engine), ('binary-helper', args.engine)]}
        count, errors = 0, []
        try:
            for label, spec in cases:
                wire = (json.dumps(dict(spec, retainAllFrames=dense), separators=(',', ':'))+'\n').encode()
                replies = {}
                for name, w in workers.items():
                    w.stdin.write(wire)
                    w.stdin.flush()
                    replies[name] = w.stdout.readline() if name == 'baseline' else read_wire(w.stdout)
                try:
                    expected = json.loads(replies['baseline'])
                    assert replies['binary'] == replies['binary-helper']
                    decode(replies['binary']).assert_matches(expected)
                except Exception:
                    location = (args.report or Path('replay-check.json')).with_suffix('') / f'{label}-{dense}'
                    location.mkdir(parents=True, exist_ok=True)
                    (location/'input.jsonl').write_bytes(wire)
                    for name, reply in replies.items(): (location/name).write_bytes(reply)
                    raise
                count += 1
                if 'error' in expected: errors.append({'case': label, 'error': expected['error']})
            for w in workers.values():
                w.stdin.close()
                if w.wait(timeout=30): raise RuntimeError('Worker exited unsuccessfully')
        finally:
            for w in workers.values():
                if w.poll() is None: w.kill(); w.wait()
                w.stdout.close()
                if not w.stdin.closed: w.stdin.close()
        report['modes'].append({'all_frames': dense, 'equivalent_replies_per_binary_mode': count, 'errors': errors})
    text = json.dumps(report, indent=2)+'\n'
    if args.report: args.report.write_text(text, encoding='utf-8')
    print(text, end='')


if __name__ == '__main__': main()
