"""Check rank tiers, input errors, and equivalent low-stat career races.

Synthetic equivalence tests do not establish official career-race accuracy.
For stats below 1200 with no learned skills or later stamina mechanics, adding
the team bonus before motivation must match adding it to each raw input stat.
"""
import argparse
import copy
import gzip
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
TIERS = [(0, 0), (1, 50), (5, 50), (6, 30), (10, 30), (11, 20),
         (15, 20), (16, 10), (20, 10), (21, 0), (26, 0), (99999, 0)]


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--engine', type=Path, required=True)
    p.add_argument('--report', type=Path, required=True)
    p.add_argument('--native-inputs', type=Path, required=True)
    args = p.parse_args()
    rows, native = [], []
    worker = subprocess.Popen([str(args.engine.resolve())], stdin=subprocess.PIPE, stdout=subprocess.PIPE)
    def run(spec):
        worker.stdin.write((json.dumps(spec, separators=(',', ':')) + '\n').encode())
        worker.stdin.flush()
        result = json.loads(worker.stdout.readline())
        if 'error' in result:
            raise ValueError(result['error'])
        return result
    try:
        for name in ['pacemaker-handover', 'inward-passing-corridor-wide-comparison']:
            case = json.loads(gzip.decompress((ROOT / 'tests/fixtures/accuracy' / (name + '.json.gz')).read_bytes()))
            base = dict(case['input'], singleMode=True, retainAllFrames=True, experimentalLaterStaminaMechanics=False)
            for i, horse in enumerate(base['horses']):
                horse['skills'] = []
                horse['mood'] = (i % 5) - 2
                for key in ['speed', 'stamina', 'power', 'guts', 'wisdom']:
                    horse[key] = 500 + i * 7
            for rank, bonus in TIERS:
                ranked, equivalent = copy.deepcopy(base), copy.deepcopy(base)
                for i, horse in enumerate(ranked['horses']):
                    # Alternate ranked and unranked runners to exercise peer/team sums.
                    horse['teamRank'] = rank if i % 2 == 0 else 0
                    for key in ['speed', 'stamina', 'power', 'guts', 'wisdom']:
                        equivalent['horses'][i][key] += bonus if i % 2 == 0 else 0
                    equivalent['horses'][i]['teamRank'] = 0
                for seed in [100003, 107922]:
                    ranked['seed'] = equivalent['seed'] = seed
                    a, b = run(ranked), run(equivalent)
                    if a != b:
                        raise AssertionError(f'equivalent stats diverged: {name}, rank {rank}, seed {seed}')
                    rows.append(dict(runners=len(base['horses']), rank=rank, seed=seed, equivalent=True))
                if rank in (0, 1, 6, 11, 16, 21):
                    native.append(copy.deepcopy(ranked))
            for bad in [-1, .5, '1', None, True, 2147483648]:
                invalid = copy.deepcopy(base)
                invalid['horses'][0]['teamRank'] = bad
                try:
                    run(invalid)
                except ValueError as error:
                    if 'teamRank must be' not in str(error):
                        raise
                else:
                    raise AssertionError(f'invalid rank accepted: {bad!r}')
                run(base)  # Failed requests must not poison the persistent worker.
        # Keep real skills in additional prepared-API cases to exercise lotteries
        # and team-value effects with nonzero rank, including overcap raw stats.
        skilled = json.loads(gzip.decompress((ROOT / 'tests/fixtures/accuracy/pacemaker-handover.json.gz').read_bytes()))['input']
        skilled['singleMode'] = True
        for i, horse in enumerate(skilled['horses']):
            horse['teamRank'] = [1, 6, 11, 16, 21][i % 5]
            horse['teamId'] = 1 + i // 3
            horse['wisdom'] = 1201 + i * 2
        native.append(skilled)
        args.native_inputs.write_text(''.join(json.dumps(s, separators=(',', ':')) + '\n' for s in native))
        report = dict(scope='synthetic rank/stat equivalence; not official capture parity',
                      engineSha256=hashlib.sha256(args.engine.read_bytes()).hexdigest(),
                      equivalentRaces=len(rows), rejectedInvalidRanks=12, recoveryChecks=12, cases=rows)
        args.report.write_text(json.dumps(report, indent=2) + '\n')
        print(json.dumps({k: v for k, v in report.items() if k != 'cases'}))
    finally:
        worker.stdin.close()
        try:
            worker.wait(timeout=30)
        finally:
            if worker.poll() is None:
                worker.kill()
                worker.wait()
            worker.stdout.close()


if __name__ == '__main__':
    main()
