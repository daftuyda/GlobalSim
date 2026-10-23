"""Output integrity checks, independent of agreement with a captured replay.

These are interface constraints, not assertions about unobserved game physics.
In particular, no acceleration, lane-motion or HP monotonicity is assumed.
"""
from collections import Counter
import math
import struct


def finite(value):
    return type(value) in (int, float) and math.isfinite(value)


def integer(value, low, high):
    return finite(value) and value == int(value) and low <= value <= high


def audit_output(actual, runners, spec=None):
    differences, counts, examples = Counter(), Counter(), []

    def require(ok, field, **where):
        if not ok:
            differences[field] += 1
            if len(examples) < 20:
                examples.append(dict(field=field, **where))

    try:
        require(len(actual['results']) == runners, 'contract.resultCount')
        for i, result in enumerate(actual['results']):
            require(finite(result['finishTimeRaw']), 'contract.finishTime', runner=i)
        if any('finishOrder' in r for r in actual['results']):
            ranks = [r.get('finishOrder') for r in actual['results']]
            valid_ranks = (all(integer(r, 0, runners - 1) for r in ranks)
                           and sorted(ranks) == list(range(runners)))
            require(valid_ranks, 'contract.finishOrderPermutation')
            if valid_ranks:
                times = [r['finishTimeRaw'] for r in sorted(actual['results'], key=lambda r: r['finishOrder'])]
                require(all(finite(t) for t in times) and all(a <= b for a, b in zip(times, times[1:])),
                        'contract.finishOrderTimes')
        if spec is not None:
            for key in ('seed', 'playerHorseIndex'):
                require(actual.get(key) == spec[key], 'contract.inputEcho.' + key)
        frames = actual['frames']
        require(bool(frames), 'contract.emptyFrames')
        for frame_index, frame in enumerate(frames):
            counts['checkedSimulationFrames'] += 1
            require(finite(frame['t']), 'contract.timestamp', frame=frame_index)
            require(len(frame['horses']) == runners, 'contract.runnerCount', frame=frame_index)
            for i, h in enumerate(frame['horses']):
                counts['checkedSimulationRunnerSamples'] += 1
                for field in ('pos', 'speed', 'hp', 'lane'):
                    require(finite(h[field]), 'finite.' + field, frame=frame_index, runner=i)
                # Wire fields are signed bytes; don't infer a smaller gameplay enum.
                require(integer(h['temptationMode'], 0, 127), 'contract.temptationMode',
                        frame=frame_index, runner=i)
                blocker = h['blockFrontHorseIndex']
                require(integer(blocker, -1, runners - 1) and blocker != i,
                        'contract.blocker', frame=frame_index, runner=i)
        for stream in ('skillEvents', 'skillTargetEvents', 'skillDetailEvents',
                       'additionalTargetEvents', 'competitionEvents', 'conservePowerEvents'):
            for index, event in enumerate(actual.get(stream, [])):
                counts['checkedOutputEvents'] += 1
                where = dict(stream=stream, event=index)
                require(integer(event['horseIndex'], 0, runners - 1), 'contract.eventRunner', **where)
                require(finite(event['t']), 'contract.eventTime', **where)
                if stream == 'competitionEvents':
                    require(event['type'] in (4, 5), 'contract.competitionType', **where)
                if stream in ('skillEvents', 'skillTargetEvents', 'skillDetailEvents', 'additionalTargetEvents'):
                    sid = event['skillId']
                    numeric = str(sid).isascii() and str(sid).isdigit()
                    synthetic = stream == 'skillEvents' and sid == 'kakari'
                    require(numeric or synthetic, 'contract.skillId', **where)
                    if synthetic:
                        counts['syntheticRushingEvents'] += 1
                    if stream == 'skillEvents':
                        require(finite(event['duration']), 'contract.duration', **where)
                    elif stream in ('skillTargetEvents', 'additionalTargetEvents'):
                        for targets in event['abilityTargets']:
                            if targets is not None:
                                require(all(integer(j, 0, runners - 1) for j in targets),
                                        'contract.targetRunner', **where)
                    if 'detailIndex' in event:
                        require(integer(event['detailIndex'], 0, 2147483647), 'contract.detailIndex', **where)
                        if spec is not None:
                            definition = spec['skillData'].get(str(sid))
                            require(definition is not None, 'contract.eventSkillDefinition', **where)
                            if definition is not None:
                                require(integer(event['detailIndex'], 0, len(definition.get('alternatives', [])) - 1),
                                        'contract.branchIndex', **where)
        # These two streams are emitted together by activate_skill. Compare
        # identities with multiplicity; matching sets would conceal lost casts.
        # Do not infer global event order from separately serialized streams.
        if 'skillDetailEvents' in actual and 'skillTargetEvents' in actual:
            def casts(stream):
                return Counter((e['horseIndex'], str(e['skillId']),
                                struct.pack('<f', e['t']), e.get('detailIndex')) for e in actual[stream])
            require(casts('skillDetailEvents') == casts('skillTargetEvents'), 'contract.detailTargetCasts')
    except (KeyError, TypeError, ValueError, OverflowError) as error:
        require(False, 'contract.malformedOutput', error=str(error))
    return dict(passed=not differences, differences=dict(differences),
                examples=examples, counts=dict(counts))


def replay_surface(expected):
    """Inventory retained wire data without guessing unknown field semantics."""
    surface = expected.get('replaySurface')
    if surface is None:
        return {}, ['raw replay event parameters and extension inventory absent']
    counts, gaps = Counter(), []
    for event in surface['events']:
        kind, params = event['kind'], event['params']
        counts[f'eventKind.{kind}'] += 1
        if kind == 3 and len(params) == 6:
            counts['recordedSkillDetailValues'] += 1
            counts['uncomparedSkillActivationFlags'] += 1
            counts['nonzeroSkillDetails'] += params[3] != 0
            counts['nonzeroSkillActivationFlags'] += params[5] != 0
        elif kind not in (4, 5, 6) or len(params) != 1:
            counts['uncomparedEventRecords'] += 1
    if counts['uncomparedSkillActivationFlags']:
        gaps.append('skill activation flag is retained but not exposed by the engine')
    if counts['uncomparedEventRecords']:
        gaps.append('unhandled official event kinds or parameter layouts are retained but not compared')
    if any(section['size'] for section in surface['extensions']):
        gaps.append('populated replay extension sections need interpretation')
    return dict(counts), gaps
