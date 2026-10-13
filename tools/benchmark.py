"""Time a persistent worker and optionally compare every reply with a baseline.

Uses Python's standard library. Inputs are JSON-lines race specifications.
Serialization of input and parsing of output are outside the timed interval;
worker request parsing, simulation, serialization and pipe transfer are inside.
"""
import argparse
import hashlib
import json
from pathlib import Path
import statistics
import subprocess
import time


def measure(engines, wires, warmup, repeats):
    workers = {name: subprocess.Popen(
        [str(engine.resolve())], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    ) for name, engine in engines.items()}
    elapsed = {name: [] for name in engines}
    hashes = {name: [] for name in engines}
    first = {}
    try:
        for index in range(warmup + repeats * len(wires)):
            wire = wires[index % len(wires)]
            # Alternate order to reduce bias from changing machine load.
            order = list(workers.items())
            if index % 2:
                order.reverse()
            for name, worker in order:
                start = time.perf_counter()
                worker.stdin.write(wire)
                worker.stdin.flush()
                reply = worker.stdout.readline()
                duration = (time.perf_counter() - start) * 1000
                if not reply:
                    raise RuntimeError(f"{name}: worker exited without a reply")
                decoded = json.loads(reply)
                if "error" in decoded:
                    raise RuntimeError(decoded["error"])
                first.setdefault(name, duration)
                if index >= warmup:
                    elapsed[name].append(duration)
                    hashes[name].append(hashlib.sha256(reply).hexdigest())
        for worker in workers.values():
            worker.stdin.close()
            if worker.wait(timeout=30) != 0:
                raise RuntimeError("Worker exited unsuccessfully")
    finally:
        for worker in workers.values():
            if worker.poll() is None:
                worker.kill()
                worker.wait()
            worker.stdout.close()
            if not worker.stdin.closed:
                worker.stdin.close()
    return {name: {
        "runs": len(times),
        "first_request_ms": first[name],
        "mean_ms": statistics.mean(times),
        "median_ms": statistics.median(times),
        "p95_ms": sorted(times)[min(len(times)-1, int(len(times)*.95))],
        "races_per_second": 1000 / statistics.mean(times),
    } for name, times in elapsed.items()}, hashes


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine", required=True, type=Path)
    parser.add_argument("--baseline", type=Path)
    parser.add_argument("--inputs", required=True, type=Path)
    parser.add_argument("--repeats", type=int, default=3)
    parser.add_argument("--warmup", type=int, default=5)
    parser.add_argument("--full-frames", action="store_true")
    args = parser.parse_args()
    if args.repeats < 1 or args.warmup < 0:
        parser.error("repeats must be positive and warmup nonnegative")
    specs = [json.loads(line) for line in args.inputs.read_text(encoding="utf-8").splitlines() if line.strip()]
    if not specs:
        parser.error("inputs must contain at least one race")
    wires = [
        (json.dumps(dict(spec, retainAllFrames=args.full_frames), separators=(",", ":")) + "\n").encode()
        for spec in specs
    ]
    report = {"input_races": len(wires), "full_frames": args.full_frames}
    engines = {"engine": args.engine}
    if args.baseline:
        engines["baseline"] = args.baseline
    timings, hashes = measure(engines, wires, args.warmup, args.repeats)
    report.update(timings)
    if args.baseline:
        expected, actual = hashes["baseline"], hashes["engine"]
        report["identical_outputs"] = sum(a == b for a, b in zip(expected, actual))
        if expected != actual:
            raise RuntimeError("Output changed relative to baseline")
        report["speedup"] = report["baseline"]["mean_ms"] / report["engine"]["mean_ms"]
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
