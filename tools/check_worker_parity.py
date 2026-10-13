"""Compare complete worker replies, including persistent-worker data isolation.

Inputs are JSON-lines race specifications. Both replay modes are checked.
--isolation requires a captured request with skillData and equipped skills.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import subprocess


def isolation_cases(specs):
    base = next((s for s in specs if isinstance(s.get("skillData"), dict)
                 and s["skillData"] and any(h.get("skills") for h in s["horses"])), None)
    if base is None:
        raise ValueError("Isolation checks require an input with skillData and equipped skills")
    changed = copy.deepcopy(base)
    edits = 0
    for definition in changed["skillData"].values():
        for alternative in definition.get("alternatives", []):
            for effect in alternative.get("effects", []):
                if effect.get("type") in (27, 31) and effect.get("modifier", 0) > 0:
                    effect["modifier"] += 100
                    edits += 1
    if not edits:
        raise ValueError("Isolation checks need speed or acceleration skill definitions")
    omitted = copy.deepcopy(base)
    omitted.pop("skillData")
    reordered = copy.deepcopy(base)
    reordered["horses"].reverse()
    return [
        ("isolation-base", base),
        ("changed-data-same-signature", changed),
        ("restored-data", base),
        ("invalid-data", dict(base, skillData=None)),
        ("after-invalid-data", base),
        ("omitted-data", omitted),
        ("after-omitted-data", base),
        ("empty-data", dict(base, skillData={})),
        ("after-empty-data", base),
        ("signature-only-change", dict(base, dataSignature="worker-parity-signature")),
        ("reordered-runners", reordered),
    ]


def check(engines, cases, isolation):
    modes = []
    for dense in (False, True):
        workers = {name: subprocess.Popen(
            [str(path.resolve())], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
        ) for name, path in engines.items()}
        errors, hashes = [], {}
        try:
            for label, spec in cases:
                wire = (json.dumps(dict(spec, retainAllFrames=dense), separators=(",", ":")) + "\n").encode()
                replies = {}
                for name, worker in workers.items():
                    worker.stdin.write(wire)
                    worker.stdin.flush()
                    replies[name] = worker.stdout.readline()
                    if not replies[name]:
                        raise RuntimeError(f"{name} exited without a reply for {label}")
                if replies["engine"] != replies["baseline"]:
                    raise RuntimeError(f"Output mismatch: {label}, dense={dense}")
                decoded = json.loads(replies["engine"])
                if "error" in decoded:
                    errors.append({"case": label, "error": decoded["error"]})
                hashes[label] = hashlib.sha256(replies["engine"]).hexdigest()
            if isolation:
                if hashes["changed-data-same-signature"] == hashes["isolation-base"]:
                    raise RuntimeError("Changed definitions did not affect the chosen race; use a more representative input")
                for label in ("restored-data", "after-invalid-data", "after-omitted-data", "after-empty-data"):
                    if hashes[label] != hashes["isolation-base"]:
                        raise RuntimeError(f"Request isolation failed for {label}")
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
                    try:
                        worker.stdin.close()
                    except BrokenPipeError:
                        pass
        modes.append({"dense": dense, "byte_identical_replies": len(cases), "error_replies": errors})
    return modes


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine", required=True, type=Path)
    parser.add_argument("--baseline", required=True, type=Path)
    parser.add_argument("--inputs", required=True, type=Path)
    parser.add_argument("--isolation", action="store_true")
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()
    specs = [json.loads(line) for line in args.inputs.read_text(encoding="utf-8-sig").splitlines() if line.strip()]
    if not specs:
        parser.error("inputs must contain at least one race")
    cases = [(f"input-{i + 1}", spec) for i, spec in enumerate(specs)]
    if args.isolation:
        cases.extend(isolation_cases(specs))
    engines = {"engine": args.engine, "baseline": args.baseline}
    report = {"input_cases": len(specs), "isolation_cases": len(cases) - len(specs),
              "modes": check(engines, cases, args.isolation),
              "sha256": {name: hashlib.sha256(path.read_bytes()).hexdigest() for name, path in engines.items()}}
    text = json.dumps(report, indent=2) + "\n"
    if args.report:
        args.report.write_text(text, encoding="utf-8")
    print(text, end="")


if __name__ == "__main__":
    main()
