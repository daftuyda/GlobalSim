"""Inventory capture coverage without equating equipped skills with exercised branches."""
import argparse
from collections import Counter
import gzip
import hashlib
import json
from pathlib import Path
import re


class Coverage:
    def __init__(self):
        self.races = 0
        self.dimensions = {k: Counter() for k in ('runners', 'mode', 'course', 'strategy',
                                                'mechanicEvents', 'runtimeFlags')}
        self.equipped, self.activated = Counter(), Counter()
        self.conditions, self.effects, self.multi_branch = set(), set(), set()
        self.sample_states = Counter()
        self.definition_hashes = set()
        self.equipped_branches, self.observed_branches = set(), set()
        self.branch_observation_races = 0
        self.branch_conditions = {}

    def add(self, spec, expected):
        self.races += 1
        self.dimensions['runners'][str(len(spec['horses']))] += 1
        self.dimensions['mode']['career' if spec.get('singleMode') else 'practice'] += 1
        self.dimensions['course'][str(spec['courseId'])] += 1
        for flag in ('experimentalRetailInitialization', 'experimentalRetailSections',
                     'experimentalRetailRuntime', 'experimentalRetailGeometry',
                     'experimentalLaterStaminaMechanics'):
            self.dimensions['runtimeFlags'][f'{flag}={spec.get(flag, False)}'] += 1
        definitions = spec.get('skillData', {})
        versions = {}
        # A skill ID alone does not identify its version or installed definition.
        for horse in spec['horses']:
            self.dimensions['strategy'][horse['strategy']] += 1
            for entry in horse['skills']:
                sid = str(entry['skillId'])
                self.equipped[sid] += 1
                definition = definitions.get(sid)
                if definition is None:
                    continue
                version = hashlib.sha256(json.dumps(
                    definition, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
                versions[sid] = version
                self.definition_hashes.add((sid, version))
                alternatives = definition.get('alternatives', [])
                self.equipped_branches.update((sid, version, i) for i in range(len(alternatives)))
                if len(alternatives) > 1:
                    self.multi_branch.add(sid)
                for index, alternative in enumerate(alternatives):
                    self.branch_conditions[sid, version, index] = dict(
                        condition=alternative.get('condition'), precondition=alternative.get('precondition'),
                        effectTypes=sorted({e['type'] for e in alternative.get('effects', [])}))
                    for key in ('condition', 'precondition'):
                        self.conditions.update(re.findall(r'([a-zA-Z_]\w*)\s*[<>=!]', alternative.get(key) or ''))
                    self.effects.update(e['type'] for e in alternative.get('effects', []))
        self.activated.update(str(sid) for _, sid, _ in expected['skillStarts'])
        if 'replaySurface' in expected:
            self.branch_observation_races += 1
            for event in expected['replaySurface']['events']:
                if event['kind'] == 3 and len(event['params']) == 6:
                    sid, detail = str(event['params'][1]), event['params'][3]
                    if sid in versions:
                        self.observed_branches.add((sid, versions[sid], detail))
        self.dimensions['mechanicEvents'].update(str(k) for k, _, _ in expected.get('mechanicEvents', []))
        for _, horses in expected['frames']:
            for values in horses:
                if len(values) >= 6:
                    self.sample_states['samplesWithState'] += 1
                    self.sample_states['blockedSamples'] += values[5] >= 0
                    self.sample_states['rushingSamples'] += values[4] != 0

    def report(self):
        return dict(races=self.races, dimensions={k: dict(sorted(v.items())) for k, v in self.dimensions.items()},
                    equippedSkillIds=len(self.equipped), activatedSkillIds=len(self.activated),
                    equippedDefinitionVersions=len(self.definition_hashes),
                    racesWithBranchObservations=self.branch_observation_races,
                    equippedBranchVersions=len(self.equipped_branches),
                    observedBranchVersions=len(self.observed_branches),
                    equippedButUnobservedBranches=[dict(skillId=sid, definitionSha256=version, detailIndex=i,
                        **self.branch_conditions[sid, version, i])
                        for sid, version, i in sorted(self.equipped_branches - self.observed_branches)],
                    equippedButNeverObservedActivating=sorted(self.equipped.keys() - self.activated.keys()),
                    equippedMultipleBranchSkillIds=sorted(self.multi_branch),
                    equippedConditionTokens=sorted(self.conditions), equippedEffectTypes=sorted(self.effects),
                    officialStateSamples=dict(self.sample_states),
                    limitations=['equipped definitions do not prove condition, branch or effect execution',
                                 'skill ID activation alone does not identify a branch; raw detail observations do',
                                 'absence from this selection is a collection gap, not proof of an engine defect'])


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--fixtures', type=Path, default=Path(__file__).resolve().parents[1] / 'tests/fixtures/accuracy')
    p.add_argument('--report', type=Path, required=True)
    args = p.parse_args()
    coverage = Coverage()
    for path in sorted(args.fixtures.glob('*.json.gz')):
        case = json.loads(gzip.decompress(path.read_bytes()))
        coverage.add(case['input'], case['expected'])
    if not coverage.races:
        p.error('no fixtures found')
    args.report.write_text(json.dumps(coverage.report(), indent=2) + '\n', encoding='utf-8')
    print(f'Inventoried {coverage.races} captured fixtures')


if __name__ == '__main__':
    main()
