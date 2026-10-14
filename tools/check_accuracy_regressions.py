"""Check captured race outcomes, skill starts, targets and replay samples.

Uses only the standard library. Fixtures contain pre-race inputs and decoded
official observations; the engine receives only the inputs.
"""
import argparse
from collections import Counter
import gzip
import hashlib
import json
from pathlib import Path
import struct
import subprocess

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
    targets = {}
    for e in actual["skillTargetEvents"]:
        key = (e["horseIndex"], str(e["skillId"]), round(e["t"], 4))
        targets[key] = visual_recipient_mask(e["abilityTargets"], e["t"])
    for i, sid, t, mask in expected["targetMasks"]:
        observed = targets.get((i, str(sid), round(t, 4)))
        assert observed == mask & 0xffffffff, f"skill {sid} visual recipient mask differs at {t}"
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
        for i, (position, speed, hp, lane) in enumerate(horses):
            if t > finish[i]:
                continue
            samples += 1
            h = ticks[round(t, 5)][i]
            assert f32(h["pos"]) == f32(position), f"position differs at {t}, runner {i}"
            for field, value, scale in [("speed", speed, 100), ("hp", hp, 1), ("lane", lane, 10000)]:
                encoded = min(65535, int(f32(max(0., min(65535. / scale, h[field]))) * scale))
                assert encoded == round(value * scale), f"{field} differs at {t}, runner {i}"
    return samples


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine", type=Path, required=True)
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()
    fixtures = sorted((ROOT / "tests/fixtures/accuracy").glob("*.json.gz"))
    if not fixtures:
        raise SystemExit("No accuracy fixtures found")
    rows = []
    with subprocess.Popen([str(args.engine.resolve())], stdin=subprocess.PIPE,
                          stdout=subprocess.PIPE, stderr=subprocess.DEVNULL) as worker:
        for fixture in fixtures:
            case = json.loads(gzip.decompress(fixture.read_bytes()))
            spec = dict(case["input"], retainAllFrames=True)
            worker.stdin.write((json.dumps(spec, separators=(",", ":")) + "\n").encode())
            worker.stdin.flush()
            reply = worker.stdout.readline()
            try:
                samples = check(case["expected"], json.loads(reply))
                rows.append({"case": case["name"], "passed": True, "runnerSamples": samples})
            except (AssertionError, ValueError) as error:
                rows.append({"case": case["name"], "passed": False, "error": str(error)})
        worker.stdin.close()
        assert worker.wait(timeout=30) == 0, "engine exited unsuccessfully"
    report = {"engineSha256": hashlib.sha256(args.engine.read_bytes()).hexdigest(),
              "passed": sum(row["passed"] for row in rows), "total": len(rows), "cases": rows}
    if args.report:
        args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    if report["passed"] != report["total"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
