"""Check captured race outcomes, skill starts, targets and replay samples.

Uses only the standard library. Fixtures contain pre-race inputs and decoded
official observations; the engine receives only the inputs.
"""
import argparse
from contextlib import nullcontext
from collections import Counter
import gzip
import hashlib
import json
import math
from pathlib import Path
import struct
import sys
from validation_contract import audit_output, replay_surface
from validation_inputs import audit_input
from validation_runner import Worker

ROOT = Path(__file__).resolve().parents[1]


def f32(value):
    return struct.unpack("<f", struct.pack("<f", value))[0]


def recorded_skill_duration(duration, event_time):
    """Captured server wire units: stored float duration, wide product, truncation.

    Rounding the product to float32 first can turn 38399.999141693115 into
    38400 and disagree with the captured 38399. Time-zero events use -1.
    This conversion does not change the engine's gameplay effect lifetime.
    """
    return -1 if event_time <= 0 else int(f32(duration) * 10000.0)


def visual_recipient_mask(ability_targets, event_time):
    """Replay visual masks describe the first ability, not all gameplay targets."""
    if event_time <= 0:
        return 0
    if not ability_targets or ability_targets[0] is None:
        return None
    return sum(1 << i for i in set(ability_targets[0])) & 0xffffffff


def check(expected, actual):
    if "error" in actual:
        raise AssertionError(actual["error"])
    finish = [r["finishTimeRaw"] for r in actual["results"]]
    assert finish == expected["finish"], "raw finish times differ"
    starts = Counter((e["horseIndex"], str(e["skillId"]), round(e["t"], 4))
                     for e in actual["skillEvents"] if str(e["skillId"]).isdigit())
    official = Counter((i, str(sid), round(t, 4)) for i, sid, t in expected["skillStarts"])
    assert starts == official, "skill activation times differ"
    if "skillRecordedDurations" in expected:
        durations = Counter((e["horseIndex"], str(e["skillId"]), round(e["t"], 4),
                             recorded_skill_duration(e["duration"], e["t"]))
                            for e in actual["skillEvents"] if str(e["skillId"]).isdigit())
        official_durations = Counter((i, str(sid), round(t, 4), units)
                                     for i, sid, t, units in expected["skillRecordedDurations"])
        assert durations == official_durations, "recorded skill durations differ"
    targets = Counter()
    for e in actual["skillTargetEvents"]:
        key = (e["horseIndex"], str(e["skillId"]), round(e["t"], 4))
        targets[(*key, visual_recipient_mask(e["abilityTargets"], e["t"]))] += 1
    for cast, count in Counter((i, str(sid), round(t, 4), mask & 0xffffffff)
                               for i, sid, t, mask in expected["targetMasks"]).items():
        assert targets[cast] >= count, f"skill {cast[1]} visual recipient mask differs at {cast[2]}"
    if "mechanicEvents" in expected:
        mechanics = Counter((e["type"], e["horseIndex"], f32(e["t"]))
                            for e in actual.get("competitionEvents", []) if e["type"] in (4, 5))
        mechanics.update((6, e["horseIndex"], f32(e["t"]))
                         for e in actual.get("conservePowerEvents", []))
        official_mechanics = Counter((kind, i, f32(t)) for kind, i, t in expected["mechanicEvents"])
        assert mechanics == official_mechanics, "competition activation events differ"
    ticks = {round(f["t"], 5): f["horses"] for f in actual["frames"]}
    samples = 0
    for t, horses in expected["frames"]:
        assert round(t, 5) in ticks, f"missing replay sample at {t}"
        for i, values in enumerate(horses):
            position, speed, hp, lane = values[:4]
            if t > finish[i]:
                continue
            samples += 1
            h = ticks[round(t, 5)][i]
            assert f32(h["pos"]) == f32(position), f"position differs at {t}, runner {i}"
            for field, value, scale in [("speed", speed, 100), ("hp", hp, 1), ("lane", lane, 10000)]:
                encoded = min(65535, int(f32(max(0., min(65535. / scale, h[field]))) * scale))
                assert encoded == round(value * scale), f"{field} differs at {t}, runner {i}"
    return samples


def check_strict(expected, actual, *, dense_frames=False, spec=None):
    """Report observation agreement separately from output integrity and coverage.

    Set dense_frames only when the caller retained every engine tick. It is an
    explicit caller assertion: a sampled frame list cannot prove its own scope.
    """
    contract = audit_output(actual, len(expected.get('finish', [])), spec)
    try:
        result = _check_observations(expected, actual)
    except (AssertionError, ValueError, KeyError, IndexError, TypeError, OverflowError) as error:
        result = dict(passed=False, differences={'malformedObservations': 1},
                      examples=[dict(error=str(error))], counts={}, coverageGaps=[])
    result['observationsPassed'] = result['passed'] if contract['passed'] else None
    result['outputContractPassed'] = contract['passed']
    result['differences'].update(contract['differences'])
    result['examples'] = (result['examples'] + contract['examples'])[:20]
    result['counts'].update(contract['counts'])
    result['passed'] = result['passed'] and contract['passed']
    try:
        surface, gaps = replay_surface(expected)
    except (KeyError, TypeError, ValueError) as error:
        surface, gaps = {}, ['malformed raw replay surface inventory']
        result['passed'] = False
        result['differences']['malformedReplaySurface'] = 1
    result['surfaceCounts'] = surface
    result['coverageGaps'].extend(gaps)
    if not dense_frames:
        result['coverageGaps'].append('all-tick output integrity not established: caller did not declare dense frames')
    if contract['counts'].get('syntheticRushingEvents'):
        result['coverageGaps'].append('synthetic rushing event durations lack an independent official event oracle')
    result['frameScope'] = 'all-engine-ticks' if dense_frames else 'supplied-frames-only'
    result['coverageStatus'] = 'partial' if result['coverageGaps'] else 'complete-for-recorded-surface'
    return result


def _check_observations(expected, actual):
    """Check every supplied official observation, including post-finish samples.

    Dense simulation frames are a superset of recorded replay frames. Match
    exact float32 timestamps, never interpolate or invent official cadence.
    Separate output event streams cannot prove global cross-stream ordering.
    """
    differences = Counter()
    examples = []
    counts = Counter()
    gaps = []

    def compare(field, wanted, got, **where):
        if wanted != got:
            differences[field] += 1
            if len(examples) < 20:
                if isinstance(wanted, list) and isinstance(got, list):
                    index = next((i for i, pair in enumerate(zip(wanted, got)) if pair[0] != pair[1]),
                                 min(len(wanted), len(got)))
                    where.update(index=index, expectedCount=len(wanted), actualCount=len(got))
                    wanted = wanted[index] if index < len(wanted) else None
                    got = got[index] if index < len(got) else None
                examples.append(dict(field=field, expected=wanted, actual=got, **where))

    def stamp(t):
        if not math.isfinite(t):
            raise ValueError("non-finite timestamp")
        return f32(t)

    if "error" in actual:
        return dict(passed=False, differences={"engineError": 1}, examples=[actual], counts={}, coverageGaps=[])
    try:
        check(expected, actual)
    except (AssertionError, ValueError, KeyError, IndexError, TypeError) as error:
        differences["legacyChecks"] += 1
        examples.append(dict(field="legacyChecks", error=str(error)))

    compare("resultCount", len(expected["finish"]), len(actual["results"]))
    compare("finishTimes", expected["finish"], [r["finishTimeRaw"] for r in actual["results"]])
    n = len(expected["finish"])
    ticks = {}
    previous = -math.inf
    for frame in actual["frames"]:
        t = stamp(frame["t"])
        compare("simulationTimestampOrder", True, t > previous, t=t)
        compare("simulationRunnerCount", n, len(frame["horses"]), t=t)
        previous = t
        ticks[t] = frame["horses"]
    previous = -math.inf
    auxiliary = True
    for raw_t, horses in expected["frames"]:
        t = stamp(raw_t)
        compare("officialTimestampOrder", True, t > previous, t=t)
        compare("officialRunnerCount", n, len(horses), t=t)
        previous = t
        counts["officialFrames"] += 1
        predicted = ticks.get(t)
        compare("missingFrame", True, predicted is not None, t=t)
        for i, values in enumerate(horses):
            counts["runnerSamples"] += 1
            if i < n and t > expected["finish"][i]:
                counts["postFinishSamples"] += 1
            auxiliary &= len(values) >= 6
            if predicted is None or i >= len(predicted):
                continue
            h = predicted[i]
            finite_fields = {field: math.isfinite(h[field]) for field in ("pos", "speed", "hp", "lane")}
            for field, finite in finite_fields.items():
                compare("finite." + field, True, finite, t=t, runner=i)
            if finite_fields["pos"]:
                compare("position", f32(values[0]), f32(h["pos"]), t=t, runner=i)
            for j, field, scale in [(1, "speed", 100), (2, "hp", 1), (3, "lane", 10000)]:
                if not finite_fields[field]:
                    continue
                encoded = min(65535, int(f32(max(0., min(65535. / scale, h[field]))) * scale))
                compare(field, round(values[j] * scale), encoded, t=t, runner=i)
            if len(values) >= 6:
                counts["stateSamples"] += 1
                for j, field in [(4, "temptationMode"), (5, "blockFrontHorseIndex")]:
                    compare(field, values[j], h.get(field), t=t, runner=i)
    if not expected["frames"]:
        differences["emptyOfficialFrames"] += 1
    if not auxiliary:
        gaps.append("rushing state and blocker identity absent from some fixture frames")

    skills = [e for e in actual["skillEvents"] if str(e["skillId"]).isdigit()]
    identity = lambda e: (e["horseIndex"], str(e["skillId"]), stamp(e["t"]))
    official_skills = [(i, str(sid), stamp(t)) for i, sid, t in expected["skillStarts"]]
    compare("skillMultiset", sorted(Counter(official_skills).items()),
            sorted(Counter(identity(e) for e in skills).items()))
    compare("skillSequence", official_skills, [identity(e) for e in skills])
    counts["skillEvents"] = len(official_skills)
    if "skillRecordedDurations" in expected:
        compare("skillDurationSequence",
                [(i, str(sid), stamp(t), units) for i, sid, t, units in expected["skillRecordedDurations"]],
                [(*identity(e), recorded_skill_duration(e["duration"], e["t"])) for e in skills])
    else:
        gaps.append("recorded skill durations absent")
    # Compare the full target multiset, retaining duplicate casts and extra events.
    expected_targets = Counter((i, str(sid), stamp(t), mask & 0xffffffff)
                              for i, sid, t, mask in expected["targetMasks"])
    target_events = actual["skillTargetEvents"]
    # Older fixtures intentionally omit all time-zero recipient sentinels.
    target_has_zero = expected.get("targetMasksComplete", False) or any(t <= 0 for _, _, t, _ in expected["targetMasks"])
    if not target_has_zero:
        gaps.append("time-zero visual recipient sentinels absent")
        target_events = [e for e in target_events if e["t"] > 0]
    actual_targets = Counter((*identity(e), visual_recipient_mask(e["abilityTargets"], e["t"]))
                             for e in target_events if str(e["skillId"]).isdigit())
    expected_target_ids = Counter((i, str(sid), stamp(t)) for i, sid, t, _ in expected["targetMasks"])
    complete_targets = expected.get("targetMasksComplete", False) or expected_target_ids == Counter(
        item for item in official_skills if target_has_zero or item[2] > 0)
    if complete_targets:
        compare("targetMultiset", sorted(expected_targets.items()), sorted(actual_targets.items()))
    else:
        gaps.append("visual recipient observations cover only a subset of casts")
        for key, count in expected_targets.items():
            compare("targetObservation", count, min(count, actual_targets.get(key, 0)), cast=key)
    if 'replaySurface' in expected:
        raw_skills = [e for e in expected['replaySurface']['events']
                      if e['kind'] == 3 and len(e['params']) == 6]
        official_details = [(e['params'][0], str(e['params'][1]), stamp(e['t']), e['params'][3])
                            for e in raw_skills]
        details = [(*identity(e), e['detailIndex']) for e in actual.get('skillDetailEvents', [])]
        counts['skillDetailObservations'] = len(official_details)
        compare('skillDetailMultiset', sorted(Counter(official_details).items()),
                sorted(Counter(details).items()))
        # Engine detail telemetry is grouped by runner. Only within-runner order
        # is exposed, so do not pretend to compare global cross-runner ordering.
        for i in range(n):
            compare('skillDetailRunnerSequence', [e for e in official_details if e[0] == i],
                    [e for e in details if e[0] == i], runner=i)
        # Correlate branch and target in the same record; separate multisets
        # could hide swapped targets on simultaneous casts of the same skill.
        official_detail_targets = [(*d, e['params'][4] & 0xffffffff)
                                   for d, e in zip(official_details, raw_skills)]
        detail_targets = [(*identity(e), e.get('detailIndex'),
                          visual_recipient_mask(e['abilityTargets'], e['t']))
                         for e in actual['skillTargetEvents']]
        compare('skillDetailTargetMultiset', sorted(Counter(official_detail_targets).items()),
                sorted(Counter(detail_targets).items()))
    else:
        gaps.append('skill branch/detail observations absent')
    if "mechanicEvents" in expected:
        for kinds, events in [((4, 5), actual.get("competitionEvents", [])),
                              ((6,), [dict(e, type=6) for e in actual.get("conservePowerEvents", [])])]:
            compare("mechanicSequence" + str(kinds),
                    [(kind, i, stamp(t)) for kind, i, t in expected["mechanicEvents"] if kind in kinds],
                    [(e["type"], e["horseIndex"], stamp(e["t"])) for e in events if e["type"] in kinds])
    else:
        gaps.append("competition event observations absent")
    if "resultFields" in expected:
        compare("officialResultCount", n, len(expected["resultFields"]))
        for i, fields in enumerate(expected["resultFields"]):
            for key, value in fields.items():
                counts["supplementaryResultValues"] += 1
                got = actual["results"][i].get(key) if i < len(actual["results"]) else None
                compare("result." + key, value, got, runner=i)
    else:
        gaps.append("supplementary result fields absent")
    if expected.get("unexposedResultFields"):
        gaps.append("official result fields not exposed by the engine: " + ", ".join(expected["unexposedResultFields"]))
    gaps.extend(["global ordering between separate event streams is not exposed",
                 "official recording cadence/count not reconstructed from dense simulation output"])
    return dict(passed=not differences, differences=dict(differences), examples=examples,
                counts=dict(counts), coverageGaps=gaps)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine", type=Path, required=True)
    parser.add_argument("--report", type=Path)
    parser.add_argument("--legacy", action="store_true", help="Use the earlier pre-finish audit only")
    parser.add_argument("--fixtures", type=Path, default=ROOT / "tests/fixtures/accuracy")
    parser.add_argument("--timeout", type=float, default=60, help="Deadline in seconds for each engine request")
    args = parser.parse_args()
    if args.timeout <= 0:
        parser.error('timeout must be positive')
    fixtures = sorted(args.fixtures.glob("*.json.gz"))
    if not fixtures:
        raise SystemExit("No accuracy fixtures found")
    rows = []
    fixture_receipts = {}
    log_context = args.report.with_suffix('.worker.log').open('wb') if args.report else nullcontext(sys.stderr)
    with log_context as log, Worker([str(args.engine.resolve())], log, output='json', timeout=args.timeout) as worker:
        for fixture in fixtures:
            raw = fixture.read_bytes()
            fixture_receipts[fixture.name] = hashlib.sha256(raw).hexdigest()
            case = json.loads(gzip.decompress(raw))
            spec = dict(case["input"], retainAllFrames=True)
            preflight = audit_input(spec, case['expected'])
            if not preflight['passed']:
                rows.append(dict(case=case['name'], passed=False, inputValidation=preflight))
                continue
            try:
                reply = worker.exchange((json.dumps(spec, separators=(",", ":")) + "\n").encode())
                actual = json.loads(reply)
                if args.legacy:
                    samples = check(case["expected"], actual)
                    rows.append({"case": case["name"], "passed": True, "runnerSamples": samples})
                else:
                    rows.append({"case": case["name"], 'inputValidation': preflight,
                                 **check_strict(case["expected"], actual, dense_frames=True, spec=spec)})
            except (AssertionError, ValueError, KeyError, IndexError, TypeError, OverflowError, OSError, TimeoutError) as error:
                rows.append({"case": case["name"], "passed": False, "error": str(error)})
    report = {"engineSha256": hashlib.sha256(args.engine.read_bytes()).hexdigest(),
              "fixturesSha256": hashlib.sha256(json.dumps(fixture_receipts, sort_keys=True,
                                                          separators=(',', ':')).encode()).hexdigest(),
              "mode": "legacy-prefinish" if args.legacy else "strict-observations-v2",
              "passed": sum(row["passed"] for row in rows), "total": len(rows), "cases": rows}
    if args.report:
        args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    if report["passed"] != report["total"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
