"""Build a release engine using compiler branch profiles from JSONL race requests.

Profile training records executed code paths; it does not fit race outcomes or
change the race mechanics. Choose inputs representative of the intended workload,
including its runner counts and replay settings. Requests are sent unchanged.

Requires Cargo, rustc and the matching llvm-tools-preview component. Artifacts,
profiles and logs remain in a fresh ignored target/pgo-* directory on every run.
The default build targets the compiler's host; --native additionally specializes
it for this machine's CPU and may make it unsuitable for other machines.

Workflow: https://doc.rust-lang.org/rustc/profile-guided-optimization.html
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
from replay_protocol import read_reply


ROOT = Path(__file__).resolve().parents[1]
BINARY = "race_engine_rs_v2"


def requests(paths):
    """Stream validated requests without retaining whole training corpora."""
    for path in paths:
        with path.open("rb") as source:
            for number, line in enumerate(source, 1):
                if not line.strip():
                    continue
                label = f"{path}:{number}"
                try:
                    spec = json.loads(line)
                except (ValueError, UnicodeError) as error:
                    raise RuntimeError(f"Invalid JSON request at {label}: {error}") from error
                if not isinstance(spec, dict):
                    raise RuntimeError(f"Expected a JSON object at {label}")
                yield label, line.rstrip(b"\r\n") + b"\n"


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def run_logged(command, environment, log):
    with log.open("w", encoding="utf-8") as output:
        result = subprocess.run(command, cwd=ROOT, env=environment,
                                stdout=output, stderr=subprocess.STDOUT)
    if result.returncode:
        raise RuntimeError(f"Command exited with status {result.returncode}; see {log}")


def train(engine, paths, count, environment, log, binary=False):
    with log.open("wb") as errors:
        worker = subprocess.Popen([str(engine)], cwd=ROOT, env=environment,
                                  stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                  stderr=errors)
        try:
            last_progress = time.monotonic()
            completed = 0
            for completed, (label, wire) in enumerate(requests(paths), 1):
                try:
                    worker.stdin.write(wire)
                    worker.stdin.flush()
                except BrokenPipeError as error:
                    raise RuntimeError(f"Worker stopped during {label}; see {log}") from error
                try:
                    if binary:
                        result = read_reply(worker.stdout).metadata
                    else:
                        reply = worker.stdout.readline()
                        if not reply:
                            raise RuntimeError(f"Worker returned no reply for {label}; see {log}")
                        result = json.loads(reply)
                except (ValueError, UnicodeError, EOFError) as error:
                    kind = "binary-v1" if binary else "JSON"
                    raise RuntimeError(f"Worker returned invalid {kind} for {label}; see {log}") from error
                if not isinstance(result, dict):
                    raise RuntimeError(f"Worker returned a non-object reply for {label}")
                if "error" in result:
                    raise RuntimeError(f"Worker rejected {label}: {result['error']}")
                now = time.monotonic()
                if now - last_progress >= 5 or completed == count:
                    print(f"Profiled {completed}/{count} requests", flush=True)
                    last_progress = now
            if completed != count:
                raise RuntimeError("Training input request count changed during the build")
            worker.stdin.close()
            if worker.wait(timeout=30) != 0:
                raise RuntimeError(f"Profiling worker exited unsuccessfully; see {log}")
        finally:
            if worker.poll() is None:
                worker.kill()
                worker.wait()
            worker.stdout.close()
            if not worker.stdin.closed:
                try:
                    worker.stdin.close()
                except BrokenPipeError:
                    pass


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inputs", type=Path, required=True, action="append",
                        help="JSONL race requests; repeat to combine input files")
    parser.add_argument("--output", type=Path,
                        help="Optionally copy the optimized executable to this path")
    parser.add_argument("--native", action="store_true",
                        help="Opt in to target-cpu=native for this machine")
    parser.add_argument("--features", default="",
                        help="Cargo features, e.g. fast-alloc")
    parser.add_argument("--train-binary", action="store_true",
                        help="Train on binary-v1 replies as well as JSON replies")
    parser.add_argument("--train-native", action="store_true",
                        help="Also profile prepared library calls (three runs per input)")
    parser.add_argument("--library-output", type=Path,
                        help="Optionally copy the optimized shared library to this path")
    parser.add_argument("--benchmark-output", type=Path,
                        help="Also build and copy the Rust benchmark_prepared example")
    args = parser.parse_args()
    paths = [path.resolve() for path in args.inputs]
    count = sum(1 for _ in requests(paths))
    if not count:
        parser.error("inputs must contain at least one race request")
    input_files = [{"path": str(path), "sha256": sha256(path)} for path in paths]

    rustc = os.environ.get("RUSTC", "rustc")
    cargo = os.environ.get("CARGO", "cargo")
    version = subprocess.check_output([rustc, "-vV"], cwd=ROOT, text=True)
    host = next((line.split(": ", 1)[1] for line in version.splitlines()
                 if line.startswith("host: ")), None)
    if not host:
        raise RuntimeError("rustc -vV did not report a host target")
    suffix = ".exe" if "-windows-" in host else ""
    sysroot = Path(subprocess.check_output([rustc, "--print", "sysroot"],
                                         cwd=ROOT, text=True).strip())
    profdata = sysroot / "lib" / "rustlib" / host / "bin" / ("llvm-profdata" + suffix)
    if not profdata.is_file():
        raise RuntimeError(f"Missing {profdata}; install it with: rustup component add llvm-tools-preview")

    environment = os.environ.copy()
    # Cargo gives encoded flags precedence over whitespace-separated RUSTFLAGS.
    encoded = environment.get("CARGO_ENCODED_RUSTFLAGS")
    common = (encoded.split("\x1f") if encoded else []) if encoded is not None else environment.get("RUSTFLAGS", "").split()
    if any(flag.startswith(("-Cprofile-generate", "-Cprofile-use", "profile-generate", "profile-use")) for flag in common):
        raise RuntimeError("Remove existing profile-generate/profile-use compiler flags before running this tool")
    if args.native:
        common.append("-Ctarget-cpu=native")
    environment.pop("RUSTFLAGS", None)

    target = ROOT / "target"
    target.mkdir(exist_ok=True)
    work = Path(tempfile.mkdtemp(prefix="pgo-", dir=target)).resolve()
    raw = work / "raw"
    raw.mkdir()
    environment["CARGO_TARGET_DIR"] = str(work / "build")
    command = [cargo, "build", "--release", "--locked", "--target", host,
               "--manifest-path", str(ROOT / "Cargo.toml"), "--bin", BINARY, "--lib"]
    if args.features:
        command.extend(["--features", args.features])
    if args.benchmark_output:
        command.extend(["--example", "benchmark_prepared"])
    engine = work / "build" / host / "release" / (BINARY + suffix)
    library = engine.parent / ("globalsim.dll" if "-windows-" in host else
        "libglobalsim.dylib" if "-apple-" in host else "libglobalsim.so")
    print(f"Build artifacts and logs: {work}", flush=True)
    print("Building instrumented release engine", flush=True)
    environment["CARGO_ENCODED_RUSTFLAGS"] = "\x1f".join(common + [f"-Cprofile-generate={raw}"])
    run_logged(command, environment, work / "generate.log")
    instrumented_hash = sha256(engine)

    print(f"Collecting compiler branch profiles from {count} race requests", flush=True)
    training_environment = environment.copy()
    training_environment["LLVM_PROFILE_FILE"] = str(raw / "%m-%p.profraw")
    training_environment["GLOBALSIM_OUTPUT"] = "json"
    train(engine, paths, count, training_environment, work / "worker.log")
    if args.train_binary:
        print(f"Collecting binary replay profiles from {count} race requests", flush=True)
        training_environment["GLOBALSIM_OUTPUT"] = "binary-v1"
        train(engine, paths, count, training_environment, work / "worker-binary.log", binary=True)
    if args.train_native:
        print(f"Collecting native library profiles from {count} prepared inputs", flush=True)
        native_command = [sys.executable, str(ROOT / "tools" / "train_native.py"), "--library", str(library)]
        for path in paths: native_command.extend(["--inputs", str(path)])
        run_logged(native_command, training_environment, work / "worker-native.log")
    for item in input_files:
        if sha256(Path(item["path"])) != item["sha256"]:
            raise RuntimeError(f"Training input changed during the build: {item['path']}")
    if not any(raw.glob("*.profraw")):
        raise RuntimeError(f"Worker produced no branch profiles in {raw}")
    merged = work / "merged.profdata"
    print("Merging compiler branch profiles", flush=True)
    run_logged([str(profdata), "merge", "-o", str(merged), str(raw)],
               environment, work / "merge.log")

    print("Building optimized release engine", flush=True)
    environment["CARGO_ENCODED_RUSTFLAGS"] = "\x1f".join(common + [
        f"-Cprofile-use={merged}", "-Cllvm-args=-pgo-warn-missing-function"])
    run_logged(command, environment, work / "use.log")
    destination = engine
    if args.output:
        destination = args.output.resolve()
        destination.parent.mkdir(parents=True, exist_ok=True)
        if destination != engine:
            shutil.copy2(engine, destination)
    library_destination = library
    if args.library_output:
        library_destination = args.library_output.resolve()
        library_destination.parent.mkdir(parents=True, exist_ok=True)
        if library_destination != library: shutil.copy2(library, library_destination)
    import_library = Path(str(library) + ".lib")
    if import_library.exists() and library_destination != library:
        shutil.copy2(import_library, Path(str(library_destination) + ".lib"))
    benchmark_destination = None
    if args.benchmark_output:
        benchmark_destination = args.benchmark_output.resolve()
        benchmark_destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(engine.parent / "examples" / ("benchmark_prepared" + suffix), benchmark_destination)
    report = {"host": host, "native": args.native, "rustc": version,
              "features": args.features, "trained_binary": args.train_binary,
              "trained_native": args.train_native,
              "replay_worker": training_environment.get("GLOBALSIM_REPLAY_WORKER", "0"),
              "common_rustflags": common, "inputs": input_files,
              "training_requests_per_format": count,
              "training_requests": count * (1 + int(args.train_binary) + 3 * int(args.train_native)),
              "instrumented_sha256": instrumented_hash,
              "profile_sha256": sha256(merged), "engine_sha256": sha256(engine),
              "engine": str(engine), "output": str(destination),
              "library": str(library), "library_output": str(library_destination),
              "library_sha256": sha256(library),
              "benchmark_output": str(benchmark_destination) if benchmark_destination else None}
    (work / "build-info.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"Optimized executable: {destination}", flush=True)


if __name__ == "__main__":
    try:
        main()
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"error: {error}", file=sys.stderr)
        sys.exit(1)
