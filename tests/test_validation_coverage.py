import unittest
from pathlib import Path
import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
from audit_validation_coverage import Coverage


class CoverageTests(unittest.TestCase):
    def test_equipment_and_activation_are_distinct_and_branch_execution_is_not_inferred(self):
        skill = dict(alternatives=[dict(condition='order>=2', effects=[dict(type=27)]),
                                   dict(condition='hp_per<30', effects=[dict(type=9)])])
        spec = dict(courseId=1, horses=[dict(strategy='Nige', skills=[dict(skillId='1'), dict(skillId='2')])],
                    skillData={'1': skill, '2': skill})
        expected = dict(skillStarts=[[0, '1', 0]], frames=[], mechanicEvents=[])
        coverage = Coverage()
        coverage.add(spec, expected)
        report = coverage.report()
        self.assertEqual(report['equippedButNeverObservedActivating'], ['2'])
        self.assertEqual(report['equippedMultipleBranchSkillIds'], ['1', '2'])
        self.assertEqual(report['equippedConditionTokens'], ['hp_per', 'order'])
        self.assertEqual(report['activatedSkillIds'], 1)
        self.assertEqual(report['observedBranchVersions'], 0)
        expected['replaySurface'] = dict(events=[dict(kind=3, t=0., params=[0, 1, -1, 1, 0, 0])])
        observed = Coverage()
        observed.add(spec, expected)
        self.assertEqual(observed.report()['observedBranchVersions'], 1)
        self.assertEqual(len(observed.report()['equippedButUnobservedBranches']), 3)
        missing = observed.report()['equippedButUnobservedBranches']
        branch = next(b for b in missing if b['skillId'] == '1')
        self.assertEqual(branch['condition'], 'order>=2')
        self.assertEqual(branch['effectTypes'], [27])


if __name__ == '__main__':
    unittest.main()
