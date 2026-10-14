"""Compare prepared library runs, independent handles and threads with a worker."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import subprocess
from check_worker_parity import isolation_cases
from native_engine import NativeRace
from replay_protocol import decode


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--library', type=Path, required=True)
    p.add_argument('--baseline', type=Path, required=True)
    p.add_argument('--inputs', type=Path, required=True)
    p.add_argument('--isolation', action='store_true')
    p.add_argument('--report', type=Path)
    args = p.parse_args()
    specs = [json.loads(s) for s in args.inputs.read_text(encoding='utf-8-sig').splitlines() if s.strip()]
    cases = [(f'input-{i+1}', s) for i,s in enumerate(specs)]
    extra = isolation_cases(specs) if args.isolation else []
    cases.extend(extra)
    report = {'library_sha256': hashlib.sha256(args.library.read_bytes()).hexdigest(),
        'baseline_sha256': hashlib.sha256(args.baseline.read_bytes()).hexdigest(),
        'inputs_sha256': hashlib.sha256(args.inputs.read_bytes()).hexdigest(), 'modes': []}
    worker = subprocess.Popen([str(args.baseline.resolve())], stdin=subprocess.PIPE,
        stdout=subprocess.PIPE, env=dict(os.environ, GLOBALSIM_OUTPUT='json', GLOBALSIM_REPLAY_WORKER='0'))
    def reference(spec):
        worker.stdin.write((json.dumps(spec,separators=(',',':'))+'\n').encode());worker.stdin.flush()
        return json.loads(worker.stdout.readline())
    try:
        for dense in [False, True]:
            checks = 0; errors = []; handles = []
            try:
                for label,s in cases:
                    spec = dict(s,retainAllFrames=dense)
                    seed = spec['seed']
                    seeds = [seed, (int(seed) ^ 7919), seed]
                    race = None
                    for index,seed in enumerate(seeds):
                        expected = reference(dict(spec,seed=seed))
                        try:
                            if race is None:
                                race = NativeRace(args.library,spec);handles.append(race)
                            raw = race.run_wire(seed)
                            actual = decode(raw)
                        except ValueError as error:
                            assert expected.get('error')==str(error),(label,index,expected.get('error'),str(error))
                            errors.append({'case':label,'seed':seed,'error':str(error)})
                        else:
                            try: actual.assert_matches(expected)
                            except Exception:
                                location=(args.report or Path('native-check.json')).with_suffix('')/f'{label}-{dense}-{index}'
                                location.mkdir(parents=True,exist_ok=True)
                                (location/'input.json').write_text(json.dumps(dict(spec,seed=seed)))
                                (location/'expected.json').write_text(json.dumps(expected))
                                (location/'actual.bin').write_bytes(raw)
                                raise
                        checks += 1
                    # Keep a few differently configured handles alive and return
                    # to each after another handle has installed its own table.
                    if len(handles)>4: handles.pop(0).close()
                report['modes'].append({'all_frames':dense,'equivalent_runs':checks,'errors':errors})
            finally:
                for handle in handles:handle.close()
        if extra:
            choices=[dict(s,retainAllFrames=True) for name,s in extra if name in ('isolation-base','changed-data-same-signature','reordered-runners')]
            refs=[reference(s) for s in choices]
            handles=[NativeRace(args.library,s) for s in choices]
            try:
                previous=[h.run(s['seed']) for h,s in zip(handles,choices)]
                for _ in range(4):
                    for h,s,ref in zip(handles,choices,refs):h.run(s['seed']).assert_matches(ref)
                for reply,ref in zip(previous,refs):reply.assert_matches(ref)
                with ThreadPoolExecutor(1) as pool:
                    try:pool.submit(handles[0].run_wire,choices[0]['seed']).result()
                    except RuntimeError as error:assert 'creating thread' in str(error)
                    else:raise AssertionError('Cross-thread handle accepted')
            finally:
                for h in handles:h.close()
            def concurrent(case):
                spec,ref=case
                with NativeRace(args.library,spec) as h:
                    for _ in range(8):h.run(spec['seed']).assert_matches(ref)
                return 8
            with ThreadPoolExecutor(3) as pool:
                report['concurrent_equivalent_runs']=sum(pool.map(concurrent,zip(choices,refs)))
            with NativeRace(args.library,choices[0]) as h:
                try:h.run_wire(1<<40)
                except ValueError:pass
                else:raise AssertionError('Out-of-range retail seed accepted')
                assert h._lib.globalsim_size(h._handle)==0
                h.run(choices[0]['seed']).assert_matches(refs[0])
                try:h.prepare(b'{invalid')
                except ValueError:pass
                else:raise AssertionError('Malformed input accepted')
                try:h.run_wire(choices[0]['seed'])
                except ValueError as error:assert 'Prepare a race' in str(error)
                else:raise AssertionError('Failed preparation retained an old race')
                h.prepare(choices[0]);h.run(choices[0]['seed']).assert_matches(refs[0])
            report['handle_isolation_and_recovery']=True
        worker.stdin.close();assert worker.wait(timeout=30)==0
    finally:
        if worker.poll() is None:worker.kill();worker.wait()
        worker.stdout.close()
    text=json.dumps(report,indent=2)+'\n'
    if args.report:args.report.write_text(text,encoding='utf-8')
    print(text,end='')


if __name__=='__main__':main()
