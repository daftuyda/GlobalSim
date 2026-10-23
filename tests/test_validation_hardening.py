"""Negative controls for capture preflight, output integrity and worker recovery."""
import copy
import gzip
import json
from pathlib import Path
import struct
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
from validation_inputs import audit_input
from validation_contract import audit_output
from validation_runner import Worker, repeatability, save_case


def spec():
    horse = dict(speed=1000, stamina=800, power=900, guts=500, wisdom=600,
                 skills=[dict(skillId='1')], gateIndex=0)
    return dict(seed=42, courseId=1, playerHorseIndex=0, singleMode=False,
                horses=[horse], skillData={'1': dict(alternatives=[dict(condition='order==1', effects=[])])})


class InputTests(unittest.TestCase):
    def test_manifest_identity_and_corrupt_inputs(self):
        expected = dict(finish=[100.])
        identity = dict(seed=42, course=1, runners=1, corpus='practice')
        self.assertTrue(audit_input(spec(), expected, identity)['passed'])
        for field, wrong in [('seed', 43), ('course', 2), ('runners', 9), ('corpus', 'career')]:
            with self.subTest(field=field):
                self.assertFalse(audit_input(spec(), expected, dict(identity, **{field: wrong}))['passed'])
        for mutation, field in [
            (lambda s: s['horses'][0].update(gateIndex=1), 'input.gates'),
            (lambda s: s.update(playerHorseIndex=1), 'input.playerIndex'),
            (lambda s: s['skillData'].clear(), 'input.missingSkillDefinition'),
            (lambda s: s['horses'][0].update(wisdom=float('nan')), 'input.nonfinite'),
            (lambda s: s.update(seed=True), 'input.seed'),
        ]:
            value = spec()
            mutation(value)
            self.assertIn(field, audit_input(value, expected)['differences'])
        self.assertIn('input.replayRunnerCount', audit_input(spec(), dict(finish=[]))['differences'])

    def test_duplicate_gate_is_not_a_valid_roster(self):
        value = spec()
        value['horses'].append(copy.deepcopy(value['horses'][0]))
        self.assertIn('input.gates', audit_input(value, dict(finish=[1., 1.]))['differences'])


def output():
    event = dict(horseIndex=0, skillId=1, t=0., detailIndex=0)
    return dict(seed=42, playerHorseIndex=0, results=[dict(finishTimeRaw=1., finishOrder=0)],
                frames=[dict(t=0., horses=[dict(pos=0., speed=0., hp=100., lane=0.,
                                               temptationMode=0, blockFrontHorseIndex=-1)])],
                skillDetailEvents=[event], skillTargetEvents=[dict(event, abilityTargets=[[]])])


class OutputTests(unittest.TestCase):
    def test_input_echo_and_branch_bounds(self):
        self.assertTrue(audit_output(output(), 1, spec())['passed'])
        value = output()
        value['seed'] += 1
        self.assertIn('contract.inputEcho.seed', audit_output(value, 1, spec())['differences'])
        value = output()
        for stream in ('skillDetailEvents', 'skillTargetEvents'):
            value[stream][0]['detailIndex'] = 1
        # Both streams agree with one another but refer to a nonexistent branch.
        self.assertIn('contract.branchIndex', audit_output(value, 1, spec())['differences'])

    def test_duplicate_cast_and_invalid_finish_permutation(self):
        value = output()
        value['skillDetailEvents'] *= 2
        self.assertIn('contract.detailTargetCasts', audit_output(value, 1)['differences'])
        value = output()
        value['results'][0]['finishOrder'] = True
        self.assertIn('contract.finishOrderPermutation', audit_output(value, 1)['differences'])
        value['results'] = [dict(finishTimeRaw=1., finishOrder=0)] * 2
        self.assertIn('contract.finishOrderPermutation', audit_output(value, 2)['differences'])
        value['results'] = [dict(finishTimeRaw=1., finishOrder=1), dict(finishTimeRaw=2., finishOrder=0)]
        self.assertIn('contract.finishOrderTimes', audit_output(value, 2)['differences'])


FAKE = '''import json, os, struct, sys, time
for line in sys.stdin.buffer:
    request = json.loads(line)
    if request.get('hang'): time.sleep(30)
    if request.get('crash'): sys.exit(7)
    if request.get('malformed'):
        sys.stdout.buffer.write(b'bad\\n'); sys.stdout.buffer.flush(); continue
    value = request.get('value', 1)
    if request.get('workerBug') and os.getenv('GLOBALSIM_REPLAY_WORKER') == '1': value += 1
    meta = {'value': value}
    if os.getenv('GLOBALSIM_OUTPUT') == 'binary-v1':
        raw = json.dumps(meta).encode()
        reply = struct.pack('<8sQIIQ', b'GSIMRP01', len(raw), 0, 0, 0) + raw
    else: reply = (json.dumps(dict(meta, frames=[])) + '\\n').encode()
    sys.stdout.buffer.write(reply); sys.stdout.buffer.flush()
'''


class WorkerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.script = self.root / 'worker.py'
        self.script.write_text(FAKE, encoding='utf-8')

    def tearDown(self):
        self.temp.cleanup()

    def test_timeout_crash_and_malformed_reply_do_not_poison_next_request(self):
        with (self.root / 'worker.log').open('wb') as log, Worker(
                [sys.executable, '-u', str(self.script)], log, timeout=.5, output='json') as worker:
            for request in (dict(hang=True), dict(crash=True), dict(malformed=True)):
                with self.subTest(request=request):
                    with self.assertRaises((TimeoutError, ValueError)):
                        worker.exchange((json.dumps(request) + '\n').encode())
                    self.assertIsNone(worker.process)
                    self.assertEqual(json.loads(worker.exchange(b'{"value":9}\n'))['value'], 9)

    def test_failure_bundle_keeps_complete_input_and_wire_bytes(self):
        directory = self.root / 'failure'
        wire = json.dumps(spec()).encode()
        save_case(directory, row=dict(passed=False), wire=wire, expected=dict(finish=[1]), reply=b'broken')
        self.assertEqual(gzip.decompress((directory / 'input.json.gz').read_bytes()), wire)
        self.assertEqual(gzip.decompress((directory / 'actual.bin.gz').read_bytes()), b'broken')
        self.assertEqual(len(json.loads((directory / 'audit.json').read_bytes())['artifacts']), 3)

    def test_repeatability_detects_json_worker_divergence(self):
        # Adapt only the executable launch; exercise the real worker, parser,
        # permutations, duplicate requests and both output formats.
        from unittest.mock import patch
        original = Worker
        def launch(command, *args, **kwargs):
            return original([sys.executable, '-u', str(self.script)], *args, **kwargs)
        # Match the fake engine's canonical binary JSON spacing.
        metadata = b'{"value": 1}'
        baseline = struct.pack('<8sQIIQ', b'GSIMRP01', len(metadata), 0, 0, 0) + metadata
        samples = [dict(key='stable', wire=b'{}\n', reply=baseline),
                   dict(key='mode-bug', wire=b'{"workerBug":true}\n', reply=baseline)]
        with patch('validation_runner.Worker', side_effect=launch):
            report = repeatability(self.script, samples, self.root / 'repeat', rounds=2)
        self.assertFalse(report['passed'])
        failures = [r for r in report['cases'] if not r['passed']]
        self.assertTrue(failures)
        self.assertTrue(all(r['key'] == 'mode-bug' and r['replayWorker'] for r in failures))
        self.assertEqual(report['checks'], 12)


if __name__ == '__main__':
    unittest.main()
