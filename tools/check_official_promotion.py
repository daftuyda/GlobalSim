"""Require unchanged official validation coverage and no practice regressions.

Third-party scores are deliberately not inputs to this gate. A fixture-only
screen can reject a candidate, but cannot approve it without the frozen corpus.
"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path

REQUIRED_VALIDATORS = {'check_accuracy_regressions.py', 'check_binary_accuracy.py',
                       'validation_inputs.py', 'validation_contract.py',
                       'validation_runner.py', 'replay_protocol.py'}


def checksum(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def load(path):
    return json.loads(Path(path).read_bytes())


def json_rows(path):
    with Path(path).open(encoding='utf-8') as stream:
        for line in stream:
            yield json.loads(line)


def fixture_rows(report):
    if report.get('mode') != 'strict-observations-v2':
        raise ValueError('Official fixtures must use strict observation validation')
    identity = report.get('fixturesSha256')
    if not isinstance(identity, str) or len(identity) != 64 or any(c not in '0123456789abcdef' for c in identity):
        raise ValueError('Official fixture report lacks a captured-input inventory receipt')
    rows = {}
    for row in report['cases']:
        if row['case'] in rows or type(row.get('passed')) is not bool:
            raise ValueError('Duplicate fixture or invalid verdict')
        if not row.get('inputValidation', {}).get('passed') or 'error' in row:
            raise ValueError('Fixture input or execution validation failed')
        rows[row['case']] = row['passed']
    if not rows or report['total'] != len(rows) or report['passed'] != sum(rows.values()):
        raise ValueError('Fixture totals do not match the recorded cases')
    return rows


def coverage_signature(row):
    # Dense simulator frame counts may legitimately change when fixing a race;
    # independently observed official coverage must not shrink or change.
    counts = {k: v for k, v in row.get('counts', {}).items()
              if k not in ('checkedSimulationFrames', 'checkedSimulationRunnerSamples', 'checkedOutputEvents')}
    value = [counts, row.get('surfaceCounts', {}), sorted(row.get('coverageGaps', []))]
    return hashlib.sha256(json.dumps(value, sort_keys=True).encode()).hexdigest()


def read_audit(directory, inventory, inventory_hash, fixture_engine):
    directory = Path(directory)
    config, summary = load(directory / 'config.json'), load(directory / 'summary.json')
    if not (config['jobsSha256'] == inventory_hash == checksum(directory / 'jobs.jsonl')):
        raise ValueError('Audit inventory differs from the required frozen inventory')
    if config['total'] != len(inventory) or summary['expected'] != len(inventory):
        raise ValueError('Audit population differs from the frozen inventory')
    if summary['configSha256'] != checksum(directory / 'config.json'):
        raise ValueError('Audit configuration receipt changed')
    if summary['resultsSha256'] != checksum(directory / 'results.jsonl'):
        raise ValueError('Audit results receipt changed or audit is still running')
    if not (summary['engineSha256'] == config['engineSha256'] == fixture_engine
            == checksum(config['engine'])):
        raise ValueError('Fixture and corpus checks must use the same pinned engine')
    if config['courseDataSha256'] != checksum(config['courseData']):
        raise ValueError('Course data changed after the audit')
    for projection in config['projections'].values():
        if projection['sha256'] != checksum(projection['path']):
            raise ValueError('Input projection changed after the audit')
    seen, active = set(), {}
    for row in json_rows(directory / 'results.jsonl'):
        key = row['replaySha256']
        if key in seen or key not in inventory:
            raise ValueError('Duplicate or unexpected corpus result')
        seen.add(key)
        expected = inventory[key]
        if (row['corpus'], row['quarantined']) != expected:
            raise ValueError('Capture type or quarantine membership changed')
        if expected != ('practice', False):
            continue
        if type(row.get('passed')) is not bool or 'error' in row:
            raise ValueError('Active practice contains invalid verdicts or input/execution errors')
        if not row.get('inputSha256') or not row.get('outputSha256') or not row.get('counts'):
            raise ValueError('Practice comparison lacks input, output or coverage evidence')
        active[key] = (row['passed'], row['inputSha256'], row['outputSha256'], coverage_signature(row))
    expected_active = {k for k, v in inventory.items() if v == ('practice', False)}
    if set(active) != expected_active or len(seen) != summary['total']:
        raise ValueError('Active practice audit is incomplete or its totals changed')
    # Paths can differ between audit directories; the checked code and assets cannot.
    tools = {}
    for path, digest in config['tools'].items():
        name = Path(path).name
        if name in tools or checksum(path) != digest:
            raise ValueError('Validator dependency changed or has an ambiguous identity')
        tools[name] = digest
    if not REQUIRED_VALIDATORS.issubset(tools):
        raise ValueError('Audit does not pin every required official validator')
    contract = dict(tools=tools, flags=config['flags'], course=config['courseDataSha256'],
                    projections={k: v['sha256'] for k, v in config['projections'].items()})
    return active, contract


def assess(baseline_fixtures, candidate_fixtures, *, inventory_path=None,
           baseline_practice=None, candidate_practice=None):
    report = dict(schema='globalsim-official-promotion-gate-v1', officialEvidenceGatePassed=False,
                  blockers=[], thirdPartyScoresUsed=False)
    try:
        before_report, after_report = load(baseline_fixtures), load(candidate_fixtures)
        before, after = fixture_rows(before_report), fixture_rows(after_report)
        if before.keys() != after.keys() or before_report['fixturesSha256'] != after_report['fixturesSha256']:
            raise ValueError('Official fixture populations differ')
        report['fixtures'] = dict(total=len(after), baselinePassed=sum(before.values()),
                                  candidatePassed=sum(after.values()),
                                  failures=sorted(k for k, passed in after.items() if not passed))
        if not all(after.values()):
            report['blockers'].append('Candidate fails official regression fixtures')
        if not all((inventory_path, baseline_practice, candidate_practice)):
            report['blockers'].append('Complete frozen active-practice comparison is required')
            return report
        inventory = {}
        for row in json_rows(inventory_path):
            key = row['replaySha256']
            if key in inventory or type(row.get('quarantined')) is not bool:
                raise ValueError('Invalid or duplicate frozen inventory entry')
            inventory[key] = (row['corpus'], row['quarantined'])
        if not any(v == ('practice', False) for v in inventory.values()):
            raise ValueError('Frozen inventory has no active official practice races')
        digest = checksum(inventory_path)
        before, before_contract = read_audit(baseline_practice, inventory, digest, before_report['engineSha256'])
        after, after_contract = read_audit(candidate_practice, inventory, digest, after_report['engineSha256'])
        if before_contract != after_contract:
            raise ValueError('Validator, flags, course data or input projections differ')
        counts, regressions, review = Counter(), [], []
        for key, old in before.items():
            new = after[key]
            if old[1] != new[1]:
                raise ValueError('Projected official input changed: ' + key)
            if old[3] != new[3]:
                raise ValueError('Observed official validation coverage changed: ' + key)
            counts.update(races=1, baselinePassed=old[0], candidatePassed=new[0],
                          fixes=not old[0] and new[0], unchangedOutputs=old[2] == new[2])
            if old[0] and not new[0]:
                regressions.append(key)
            if not old[0] and not new[0] and old[2] != new[2]:
                review.append(key)
        report['practice'] = dict(counts, inventorySha256=digest, regressions=regressions,
                                  changedUnresolvedCases=review)
        if regressions:
            report['blockers'].append('Candidate regresses previously matching official practice captures')
        if review:
            report['blockers'].append('Changed unresolved official cases require investigation')
        report['officialEvidenceGatePassed'] = not report['blockers']
    except (ValueError, KeyError, TypeError, OSError) as error:
        report['blockers'].append(str(error))
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--baseline-fixtures', type=Path, required=True)
    parser.add_argument('--candidate-fixtures', type=Path, required=True)
    parser.add_argument('--inventory', type=Path, help='Authoritative frozen corpus jobs.jsonl')
    parser.add_argument('--baseline-practice', type=Path, help='Frozen full-corpus audit directory')
    parser.add_argument('--candidate-practice', type=Path, help='Frozen full-corpus audit directory')
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args()
    result = assess(args.baseline_fixtures, args.candidate_fixtures, inventory_path=args.inventory,
                    baseline_practice=args.baseline_practice, candidate_practice=args.candidate_practice)
    args.report.write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(result))
    if not result['officialEvidenceGatePassed']:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
