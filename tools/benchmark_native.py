"""Compare full binary replies from a worker and a prepared in-process library.

Preparation is reported separately. Library timing includes binary encoding and
copying every reply byte into Python; caller request encoding/decoding is excluded.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import statistics
import subprocess
import time
from native_engine import NativeRace
from replay_protocol import decode, read_wire


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--library', type=Path, required=True)
    p.add_argument('--baseline', type=Path, required=True)
    p.add_argument('--inputs', type=Path, required=True)
    p.add_argument('--repeats', type=int, default=7)
    p.add_argument('--warmup', type=int, default=5)
    p.add_argument('--mc-seeds', type=int, default=0, help='Generate this many seeds from a single input roster')
    p.add_argument('--report', type=Path)
    args = p.parse_args()
    if args.repeats < 1 or args.warmup < 0 or args.mc_seeds < 0: p.error('Invalid run counts')
    specs = [json.loads(s) for s in args.inputs.read_text(encoding='utf-8-sig').splitlines() if s.strip()]
    if not specs: p.error('Inputs must contain race requests')
    if args.mc_seeds:
        if len(specs) != 1: p.error('--mc-seeds requires a single input roster')
        specs = [dict(specs[0],seed=100003+i*7919) for i in range(args.mc_seeds)]
    specs = [dict(s,retainAllFrames=True) for s in specs]
    wires = [(json.dumps(s,separators=(',',':'))+'\n').encode() for s in specs]
    worker = subprocess.Popen([str(args.baseline.resolve())], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
        env=dict(os.environ,GLOBALSIM_OUTPUT='binary-v1',GLOBALSIM_REPLAY_WORKER='0'))
    handles = {}; keys = []; preparations = []; timings = {'worker': [], 'prepared': []}
    digests = set(); sizes = []
    try:
        for spec in specs:
            key = json.dumps({k:v for k,v in spec.items() if k!='seed'},sort_keys=True,separators=(',',':'))
            keys.append(key)
            if key not in handles:
                start=time.perf_counter_ns();handles[key]=NativeRace(args.library,spec)
                preparations.append((time.perf_counter_ns()-start)/1e6)
        for index in range(args.warmup+args.repeats*len(specs)):
            ci=index%len(specs);replies={}
            for mode in (['worker','prepared'] if index%2 else ['prepared','worker']):
                start=time.perf_counter_ns()
                if mode=='worker':
                    worker.stdin.write(wires[ci]);worker.stdin.flush();reply=read_wire(worker.stdout)
                else: reply=handles[keys[ci]].run_wire(specs[ci]['seed'])
                elapsed=(time.perf_counter_ns()-start)/1e6
                if index>=args.warmup:timings[mode].append(elapsed)
                replies[mode]=reply
            if replies['prepared']!=replies['worker']:raise RuntimeError(f'Output differs at input {ci}')
            digest=hashlib.sha256(replies['worker']).hexdigest()
            if digest not in digests:
                result=decode(replies['worker'])
                if 'error' in result.metadata:raise RuntimeError(result.metadata['error'])
                digests.add(digest)
            if index>=args.warmup:sizes.append(len(replies['worker']))
        worker.stdin.close()
        if worker.wait(timeout=30):raise RuntimeError('Reference worker exited unsuccessfully')
    finally:
        for h in handles.values():h.close()
        if worker.poll() is None:worker.kill();worker.wait()
        worker.stdout.close()
    report={'scope':__doc__,'repeats':args.repeats,'warmup':args.warmup,'preparation_ms':preparations,
        'prepared_rosters':len(preparations),'all_frames':True,'distinct_compared_replies':len(digests),
        'mean_reply_bytes':statistics.mean(sizes),'inputs_sha256':hashlib.sha256(b''.join(wires)).hexdigest(),
        'sha256':{k:hashlib.sha256(v.read_bytes()).hexdigest() for k,v in [('library',args.library),('baseline',args.baseline)]},
        'timings':{k:{'runs':len(v),'mean_ms':statistics.mean(v),'median_ms':statistics.median(v),
            'p95_ms':sorted(v)[int(len(v)*.95)],'samples_ms':v} for k,v in timings.items()}}
    text=json.dumps(report,indent=2)+'\n'
    if args.report:args.report.write_text(text,encoding='utf-8')
    print(text,end='')


if __name__=='__main__':main()
