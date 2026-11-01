"""Prevent third-party gains, missing captures or weaker checks from approving a build."""
import copy
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
from check_official_promotion import REQUIRED_VALIDATORS, assess, checksum


def save(path, value):
    path.write_text(json.dumps(value), encoding='utf-8')


class OfficialPromotionTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.inventory = self.root / 'inventory.jsonl'
        self.jobs = [dict(replaySha256=k, corpus=c, quarantined=q) for k, c, q in
                     [('a', 'practice', False), ('b', 'practice', False),
                      ('q', 'practice', True), ('c', 'career', False)]]
        self.inventory.write_text(''.join(json.dumps(j) + '\n' for j in self.jobs))
        self.asset = self.root / 'asset.json'
        self.asset.write_text('{}')
        self.validators = [self.root / name for name in sorted(REQUIRED_VALIDATORS)]
        for path in self.validators:
            path.write_text('# frozen validator\n')
        self.validator = self.validators[0]
        self.base = self.root / 'baseline'; self.candidate = self.root / 'candidate'
        for directory in (self.base, self.candidate):
            directory.mkdir()
            (directory / 'engine.exe').write_bytes(directory.name.encode())
            (directory / 'jobs.jsonl').write_bytes(self.inventory.read_bytes())
            engine_hash = checksum(directory / 'engine.exe')
            save(directory / 'fixtures.json', dict(mode='strict-observations-v2', total=1, passed=1,
                 fixturesSha256='f' * 64,
                 engineSha256=engine_hash, cases=[dict(case='edge', passed=True, inputValidation=dict(passed=True))]))
            config = dict(total=4, jobsSha256=checksum(self.inventory), engine=str(directory / 'engine.exe'),
                          engineSha256=engine_hash, courseData=str(self.asset), courseDataSha256=checksum(self.asset),
                          tools={str(p): checksum(p) for p in self.validators}, flags={'retail': True},
                          projections={'master': dict(path=str(self.asset), sha256=checksum(self.asset))})
            save(directory / 'config.json', config)
            rows = [dict(j, passed=j['replaySha256'] == 'a', inputSha256='input-' + j['replaySha256'],
                         outputSha256='output-' + j['replaySha256'], counts={'officialFrames': 100},
                         surfaceCounts={'finish': 9}, coverageGaps=['unrecorded ticks unavailable']) for j in self.jobs]
            self.write_rows(directory, rows)

    def write_rows(self, directory, rows):
        (directory / 'results.jsonl').write_text(''.join(json.dumps(r) + '\n' for r in rows))
        config = json.loads((directory / 'config.json').read_text())
        save(directory / 'summary.json', dict(expected=4, total=len(rows),
             engineSha256=config['engineSha256'], configSha256=checksum(directory / 'config.json'),
             resultsSha256=checksum(directory / 'results.jsonl')))

    def rows(self):
        return [json.loads(s) for s in (self.candidate / 'results.jsonl').read_text().splitlines()]

    def evaluate(self):
        return assess(self.base / 'fixtures.json', self.candidate / 'fixtures.json',
                      inventory_path=self.inventory, baseline_practice=self.base, candidate_practice=self.candidate)

    def test_fixture_only_pass_cannot_approve(self):
        result = assess(self.base / 'fixtures.json', self.candidate / 'fixtures.json')
        self.assertFalse(result['officialEvidenceGatePassed'])
        self.assertEqual(result['fixtures']['candidatePassed'], 1)

    def test_unchanged_known_failure_does_not_become_a_pass(self):
        result = self.evaluate()
        self.assertTrue(result['officialEvidenceGatePassed'], result)
        self.assertEqual(result['practice']['races'], 2)
        self.assertEqual(result['practice']['candidatePassed'], 1)

    def test_fix_is_allowed_without_counting_quarantine_or_career(self):
        rows = self.rows(); rows[1].update(passed=True, outputSha256='fixed')
        self.write_rows(self.candidate, rows)
        result = self.evaluate()
        self.assertTrue(result['officialEvidenceGatePassed'], result)
        self.assertEqual(result['practice']['fixes'], 1)
        self.assertEqual(result['practice']['candidatePassed'], 2)

    def test_fix_cannot_compensate_for_a_regression(self):
        rows = self.rows(); rows[0]['passed'] = False; rows[1]['passed'] = True
        self.write_rows(self.candidate, rows)
        result = self.evaluate()
        self.assertFalse(result['officialEvidenceGatePassed'])
        self.assertEqual(result['practice']['regressions'], ['a'])

    def test_changed_unresolved_output_requires_investigation(self):
        rows = self.rows(); rows[1]['outputSha256'] = 'different-but-still-failing'
        self.write_rows(self.candidate, rows)
        result = self.evaluate()
        self.assertFalse(result['officialEvidenceGatePassed'])
        self.assertEqual(result['practice']['changedUnresolvedCases'], ['b'])

    def test_missing_and_duplicate_cases_fail_closed(self):
        original = self.rows()
        for rows in [original[1:], original + [original[0]]]:
            with self.subTest(rows=len(rows)):
                self.write_rows(self.candidate, rows)
                self.assertFalse(self.evaluate()['officialEvidenceGatePassed'])

    def test_inputs_coverage_and_membership_cannot_change(self):
        original = self.rows()
        for update in [dict(inputSha256='altered'), dict(counts={'officialFrames': 99}),
                       dict(quarantined=True), dict(corpus='career'), dict(error='timeout'),
                       dict(coverageGaps=['additional unobserved channel'])]:
            with self.subTest(update=update):
                rows = copy.deepcopy(original); rows[0].update(update)
                self.write_rows(self.candidate, rows)
                self.assertFalse(self.evaluate()['officialEvidenceGatePassed'])

    def test_fixture_failure_blocks_an_improved_practice_cohort(self):
        rows = self.rows(); rows[1]['passed'] = True; self.write_rows(self.candidate, rows)
        path = self.candidate / 'fixtures.json'; report = json.loads(path.read_text())
        report['cases'][0]['passed'] = False; report['passed'] = 0; save(path, report)
        self.assertFalse(self.evaluate()['officialEvidenceGatePassed'])

    def test_legacy_or_different_fixture_populations_are_rejected(self):
        path = self.candidate / 'fixtures.json'; original = json.loads(path.read_text())
        for field in ['mode', 'case', 'fixturesSha256']:
            report = copy.deepcopy(original)
            if field == 'mode': report['mode'] = 'legacy-prefinish'
            elif field == 'case': report['cases'][0]['case'] = 'replacement'
            else: report['fixturesSha256'] = '0' * 64
            save(path, report)
            self.assertFalse(self.evaluate()['officialEvidenceGatePassed'])

    def test_validator_configuration_cannot_change(self):
        path = self.candidate / 'config.json'; config = json.loads(path.read_text())
        config['flags'] = {'retail': False}; save(path, config)
        self.write_rows(self.candidate, self.rows())
        self.assertFalse(self.evaluate()['officialEvidenceGatePassed'])

    def test_incomplete_validator_receipts_cannot_approve(self):
        for directory in (self.base, self.candidate):
            path = directory / 'config.json'; config = json.loads(path.read_text())
            config['tools'] = {}; save(path, config)
            rows = [json.loads(s) for s in (directory / 'results.jsonl').read_text().splitlines()]
            self.write_rows(directory, rows)
        self.assertFalse(self.evaluate()['officialEvidenceGatePassed'])

    def test_tampered_receipts_and_assets_are_rejected(self):
        for path in [self.candidate / 'engine.exe', self.candidate / 'results.jsonl',
                     self.candidate / 'jobs.jsonl', self.validator, self.asset]:
            with self.subTest(path=path.name):
                original = path.read_bytes(); path.write_bytes(original + b' ')
                self.assertFalse(self.evaluate()['officialEvidenceGatePassed'])
                path.write_bytes(original)


if __name__ == '__main__':
    unittest.main()
