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

    def test_unsampled_frame_values_cannot_hide(self):
        for field, value in [('hp', float('nan')), ('lane', float('inf')),
                             ('pos', None), ('speed', True)]:
            with self.subTest(field=field):
                expected, actual = example()
                hidden = copy.deepcopy(actual['frames'][0])
                hidden['t'] = .5
                hidden['horses'][0][field] = value
                actual['frames'].insert(1, hidden)
                check(expected, actual)
                result = check_strict(expected, actual, dense_frames=True)
                self.assertFalse(result['passed'])
                self.assertIn('finite.' + field, result['differences'])
                self.assertEqual(result['counts']['checkedSimulationFrames'], 3)

    def test_invalid_indices_and_unknown_events_cannot_be_filtered_out(self):
        mutations = [
            ('competitionEvents', dict(type=99, horseIndex=0, t=.5), 'contract.competitionType'),
            ('skillEvents', dict(skillId='mystery', horseIndex=0, t=.5, duration=1), 'contract.skillId'),
            ('skillEvents', dict(skillId=1, horseIndex=99, t=.5, duration=1), 'contract.eventRunner'),
            ('skillTargetEvents', dict(skillId=1, horseIndex=0, t=.5, abilityTargets=[[32]]), 'contract.targetRunner'),
        ]
        for stream, event, field in mutations:
            with self.subTest(field=field):
                expected, actual = example()
                actual[stream].append(event)
                self.assertIn(field, check_strict(expected, actual)['differences'])

    def test_known_rushing_event_is_allowed_but_not_called_validated(self):
        expected, actual = example()
        actual['skillEvents'].append(dict(skillId='kakari', horseIndex=0, t=.5, duration=1))
        result = check_strict(expected, actual)
        self.assertTrue(result['passed'])
        self.assertEqual(result['counts']['syntheticRushingEvents'], 1)
        self.assertEqual(result['coverageStatus'], 'partial')

    def test_raw_surface_gaps_are_visible_even_when_observations_match(self):
        expected, actual = example()
        expected['skillStarts'] = [[0, '1', 0.]]
        expected['skillRecordedDurations'] = [[0, '1', 0., -1]]
        expected['targetMasks'] = [[0, '1', 0., 0]]
        actual['skillEvents'] = [dict(horseIndex=0, skillId=1, t=0., duration=0.)]
        actual['skillDetailEvents'] = [dict(horseIndex=0, skillId=1, t=0., detailIndex=2)]
        actual['skillTargetEvents'] = [dict(horseIndex=0, skillId=1, t=0., detailIndex=2, abilityTargets=[[0]])]
        expected['replaySurface'] = dict(events=[dict(kind=3, t=0, params=[0, 1, -1, 2, 0, 1]),
                                               dict(kind=99, t=0, params=[0])],
                                        extensions=[dict(name='before_frames', size=8)])
        result = check_strict(expected, actual, dense_frames=True)
        self.assertTrue(result['passed'])
        self.assertEqual(result['surfaceCounts']['nonzeroSkillDetails'], 1)
        self.assertEqual(result['surfaceCounts']['nonzeroSkillActivationFlags'], 1)
        self.assertEqual(result['surfaceCounts']['uncomparedEventRecords'], 1)
        self.assertEqual(result['coverageStatus'], 'partial')
        self.assertTrue(any('extension' in gap for gap in result['coverageGaps']))
        actual['skillDetailEvents'][0]['detailIndex'] = 0
        result = check_strict(expected, actual)
        self.assertIn('skillDetailMultiset', result['differences'])

    def test_simultaneous_branch_targets_cannot_be_swapped(self):
        expected, actual = example()
        expected['skillStarts'] = [[0, '1', .5], [0, '1', .5]]
        expected['skillRecordedDurations'] = [[0, '1', .5, 0], [0, '1', .5, 0]]
        expected['targetMasks'] = [[0, '1', .5, 0], [0, '1', .5, 1]]
        expected['replaySurface'] = dict(events=[dict(kind=3, t=.5, params=[0, 1, 0, d, d, 0])
                                               for d in (0, 1)], extensions=[])
        actual['skillEvents'] = [dict(horseIndex=0, skillId=1, t=.5, duration=0.) for _ in (0, 1)]
        actual['skillDetailEvents'] = [dict(horseIndex=0, skillId=1, t=.5, detailIndex=d) for d in (0, 1)]
        actual['skillTargetEvents'] = [dict(horseIndex=0, skillId=1, t=.5, detailIndex=d,
                                          abilityTargets=[[] if d == 0 else [0]]) for d in (0, 1)]
        self.assertTrue(check_strict(expected, actual)['passed'])
        actual['skillTargetEvents'][0]['abilityTargets'], actual['skillTargetEvents'][1]['abilityTargets'] = (
            actual['skillTargetEvents'][1]['abilityTargets'], actual['skillTargetEvents'][0]['abilityTargets'])
        result = check_strict(expected, actual)
        self.assertNotIn('targetMultiset', result['differences'])
        self.assertIn('skillDetailTargetMultiset', result['differences'])

    def test_malformed_output_returns_failed_report(self):
        expected, actual = example()
        for bad in [None, {}, dict(actual, frames=None), dict(actual, frames=[{}])]:
            with self.subTest(bad=type(bad).__name__):
                result = check_strict(expected, bad)
                self.assertFalse(result['passed'])

    def test_dense_scope_requires_explicit_caller_assertion(self):
        expected, actual = example()
        self.assertEqual(check_strict(expected, actual)['frameScope'], 'supplied-frames-only')
        self.assertEqual(check_strict(expected, actual, dense_frames=True)['frameScope'], 'all-engine-ticks')


if __name__ == '__main__':
    unittest.main()
