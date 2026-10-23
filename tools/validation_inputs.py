"""Preflight captured race inputs without using outcomes to repair them.

This checks the normalized capture contract, not every possible engine API input.
In particular a capture roster must contain each gate exactly once.
"""
from collections import Counter
from validation_contract import finite, integer


def audit_input(spec, expected, identity=None):
    differences, examples = Counter(), []

    def require(ok, field, **where):
        if not ok:
            differences[field] += 1
            if len(examples) < 20:
                examples.append(dict(field=field, **where))

    def numbers(value, path='input'):
        if isinstance(value, dict):
            for key, child in value.items():
                numbers(child, f'{path}.{key}')
        elif isinstance(value, list):
            for index, child in enumerate(value):
                numbers(child, f'{path}[{index}]')
        elif type(value) in (int, float):
            require(finite(value), 'input.nonfinite', path=path)

    try:
        numbers(spec)
        horses = spec['horses']
        n = len(horses)
        require(n > 0, 'input.emptyRoster')
        require(integer(spec['seed'], -(2**63), 2**63 - 1), 'input.seed')
        require(integer(spec['courseId'], 1, 2**31 - 1), 'input.course')
        require(integer(spec['playerHorseIndex'], 0, n - 1), 'input.playerIndex')
        gates = [h['gateIndex'] for h in horses]
        require(all(integer(g, 0, n - 1) for g in gates) and sorted(gates) == list(range(n)),
                'input.gates')
        require(len(expected['finish']) == n, 'input.replayRunnerCount')
        for i, horse in enumerate(horses):
            for stat in ('speed', 'stamina', 'power', 'guts', 'wisdom'):
                require(finite(horse[stat]), 'input.stat', runner=i, stat=stat)
            for entry in horse['skills']:
                sid = str(entry['skillId'])
                require(sid in spec['skillData'], 'input.missingSkillDefinition', runner=i, skillId=sid)
        for field, value in (('seed', spec['seed']), ('course', spec['courseId']), ('runners', n)):
            if identity is not None and field in identity:
                require(identity[field] == value, 'input.manifest.' + field,
                        expected=identity[field], actual=value)
        if identity is not None and 'corpus' in identity:
            require(bool(spec.get('singleMode')) == (identity['corpus'] == 'career'), 'input.manifest.mode')
    except (KeyError, TypeError, ValueError, OverflowError) as error:
        require(False, 'input.malformed', error=str(error))
    return dict(passed=not differences, differences=dict(differences), examples=examples)
