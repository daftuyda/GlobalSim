"""Compare seed batches and results-only replies with independent baseline requests."""
import argparse,json,sys,hashlib
from pathlib import Path
from contextlib import ExitStack
sys.path.insert(0,str(Path(__file__).resolve().parent))
from validation_runner import Worker
from replay_protocol import decode
def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--engine',type=Path,required=True)
    parser.add_argument('--baseline',type=Path,required=True)
    parser.add_argument('--inputs',type=Path,required=True)
    parser.add_argument('--report',type=Path,required=True)
    args=parser.parse_args()
    out=args.report.resolve().parent
    out.mkdir(parents=True,exist_ok=True)
    baseline=args.baseline.resolve();candidate=args.engine.resolve()
    specs=[json.loads(s) for s in args.inputs.read_text(encoding='utf-8-sig').splitlines() if s.strip()]
    if not specs: parser.error('inputs must contain race requests')
    def wire(s):return (json.dumps(s,separators=(',',':'))+'\n').encode()
    framekeys={'frames','finishFrames','rawFrameCount','framesDownsampled','gateFrameNeeded'}
    report={'checks':0,'modes':[], 'engineSha256':hashlib.sha256(candidate.read_bytes()).hexdigest(), 'baselineSha256':hashlib.sha256(baseline.read_bytes()).hexdigest(), 'inputsSha256':hashlib.sha256(args.inputs.read_bytes()).hexdigest()}
    for fmt,helper in [('binary-v1',False),('json',False),('json',True)]:
     with ExitStack() as stack:
      log=stack.enter_context((out/f'batch-{fmt}-{helper}.worker.log').open('wb'))
      ref=stack.enter_context(Worker([str(baseline)],log,output=fmt))
      opt=stack.enter_context(Worker([str(candidate)],log,output=fmt,replay_worker=helper))
      for dense in [False,True]:
       # Every fixture exercises four-thread batching; a subset also covers serial
       # batching, results-only, malformed seeds and recovery on reused workers.
       for i,original in enumerate(specs):
        spec=dict(original,retainAllFrames=dense);seed=spec['seed'];seeds=[seed,seed^7919,seed]
        expected=[ref.exchange(wire(dict(spec,seed=s))) for s in seeds]
        for threads,omit in ([(4,False)] if i>=9 else [(1,False),(4,False),(0,False),(4,True)]):
         request=dict(spec,seeds=seeds,threads=threads,omitFrames=omit)
         for j,exp in enumerate(expected):
          actual=opt.exchange(wire(request) if j==0 else b'')
          if omit:
           if fmt=='json':
            a=json.loads(actual);e={k:v for k,v in json.loads(exp).items() if k not in framekeys}
           else:
            a=decode(actual);assert a.count==0 and a.horse_count==0;a=a.metadata
            e={k:v for k,v in decode(exp).metadata.items() if k not in framekeys}
           assert a==e,(fmt,helper,dense,i,threads,omit,j,'results-only mismatch')
          else:
           if actual!=exp:
            case=out/f'batch-failure-{fmt}-{helper}-{dense}-{i}-{threads}-{j}';case.mkdir(exist_ok=True)
            (case/'input.json').write_bytes(wire(request));(case/'actual').write_bytes(actual);(case/'expected').write_bytes(exp)
            raise AssertionError(str(case))
          report['checks']+=1
       report['modes'].append(dict(format=fmt,helper=helper,dense=dense,cumulativeChecks=report['checks']));print(report['modes'][-1],flush=True)
      spec=specs[0];seeds=[spec['seed'],1<<40,None,'bad',spec['seed']]
      expected=[ref.exchange(wire(dict(spec,seed=s))) for s in seeds]
      for i,exp in enumerate(expected):
       assert opt.exchange(wire(dict(spec,seeds=seeds,threads=4)) if i==0 else b'')==exp
       report['checks']+=1
      for extra in [dict(seeds=[]),dict(threads=-1),dict(omitFrames=1)]:
       actual=opt.exchange(wire(dict(spec,**extra)));value=json.loads(actual) if fmt=='json' else decode(actual).metadata
       assert 'error' in value; report['checks']+=1
       assert opt.exchange(wire(spec))==ref.exchange(wire(spec));report['checks']+=1
    report['passed']=True
    args.report.write_text(json.dumps(report,indent=2));print(json.dumps(report),flush=True)


if __name__ == '__main__':
    main()
