"""Mutation tests: a stricter audit must detect formerly hidden discrepancies."""
import copy
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
from check_accuracy_regressions import check, check_strict


def example():
    expected = dict(finish=[1.0], skillStarts=[], targetMasks=[],
                    targetMasksComplete=True, skillRecordedDurations=[], mechanicEvents=[],
                    frames=[[0.0, [[0., 3., 100., 0., 0, -1]]],
                            [2.0, [[25., 5., 90., .1, 0, -1]]]],
                    resultFields=[dict(startDelayTime=0.0)])
    actual = dict(results=[dict(finishTimeRaw=1.0, startDelayTime=0.0)], skillEvents=[],
                  skillTargetEvents=[], competitionEvents=[], conservePowerEvents=[],
                  frames=[dict(t=t, horses=[dict(pos=h[0], speed=h[1], hp=h[2], lane=h[3],
                                                temptationMode=h[4], blockFrontHorseIndex=h[5])
                                           for h in horses]) for t, horses in expected['frames']])
    return expected, actual


class StrictAuditTests(unittest.TestCase):
    def test_finish_time_is_explicitly_checked(self):
        expected, actual = example()
        actual['results'][0]['finishTimeRaw'] = .9
        self.assertIn('finishTimes', check_strict(expected, actual)['differences'])

    def test_postfinish_is_compared(self):
        expected, actual = example()
        result = check_strict(expected, actual)
        self.assertTrue(result['passed'])
        self.assertEqual(result['counts']['postFinishSamples'], 1)
        actual['frames'][1]['horses'][0]['hp'] -= 1
        check(expected, actual)  # Earlier pre-finish metric deliberately excludes this.
        self.assertIn('hp', check_strict(expected, actual)['differences'])

    def test_timestamp_rounding_and_duplicates_cannot_hide_samples(self):
        expected, actual = example()
        actual['frames'][0]['t'] = .000001
        check(expected, actual)
        self.assertIn('missingFrame', check_strict(expected, actual)['differences'])
        expected, actual = example()
        actual['frames'].insert(1, copy.deepcopy(actual['frames'][0]))
        self.assertIn('simulationTimestampOrder', check_strict(expected, actual)['differences'])

    def test_recorded_state_and_results(self):
        expected, actual = example()
        actual['frames'][1]['horses'][0]['blockFrontHorseIndex'] = 0
        actual['results'][0]['startDelayTime'] = .1
        result = check_strict(expected, actual)
        self.assertIn('blockFrontHorseIndex', result['differences'])
        self.assertIn('result.startDelayTime', result['differences'])

    def test_nonfinite_values_do_not_pass_wire_clamping(self):
        expected, actual = example()
        actual['frames'][1]['horses'][0]['hp'] = float('nan')
        self.assertIn('finite.hp', check_strict(expected, actual)['differences'])

    def test_event_order_and_extra_targets(self):
        expected, actual = example()
        expected['skillStarts'] = [[0, '1', 0.], [0, '2', 0.]]
        expected['skillRecordedDurations'] = [[0, '1', 0., -1], [0, '2', 0., -1]]
        actual['skillEvents'] = [dict(horseIndex=0, skillId=sid, t=0., duration=0.) for sid in [2, 1]]
        actual['skillTargetEvents'] = [dict(horseIndex=0, skillId=1, t=2., abilityTargets=[[0]])]
        check(expected, actual)
        result = check_strict(expected, actual)
        self.assertIn('skillSequence', result['differences'])
        self.assertIn('targetMultiset', result['differences'])

    def test_missing_observations_are_reported_as_coverage_gaps(self):
        expected, actual = example()
        del expected['resultFields']
        for _, horses in expected['frames']:
            for horse in horses:
                del horse[4:]
        result = check_strict(expected, actual)
        self.assertTrue(result['passed'])
        self.assertIn('supplementary result fields absent', result['coverageGaps'])
        self.assertTrue(any('blocker' in gap for gap in result['coverageGaps']))


if __name__ == '__main__':
    unittest.main()
