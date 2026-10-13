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


def check(expected, actual):
    if "error" in actual:
        raise AssertionError(actual["error"])
    finish = [r["finishTimeRaw"] for r in actual["results"]]
    assert finish == expected["finish"], "raw finish times differ"
    starts = Counter((e["horseIndex"], str(e["skillId"]), round(e["t"], 4))
                     for e in actual["skillEvents"] if str(e["skillId"]).isdigit())
    official = Counter((i, str(sid), round(t, 4)) for i, sid, t in expected["skillStarts"])
    assert starts == official, "skill activation times differ"
    targets = {}
    for e in actual["skillTargetEvents"]:
        key = (e["horseIndex"], str(e["skillId"]), round(e["t"], 4))
        targets.setdefault(key, set()).update(i for group in e["abilityTargets"] for i in (group or []))
    for i, sid, t, mask in expected["targetMasks"]:
        observed = sum(1 << j for j in targets.get((i, str(sid), round(t, 4)), set()))
        assert observed == mask, f"skill {sid} recipient differs at {t}"
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
