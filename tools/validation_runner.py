"""Bounded validation workers and repeatability probes; never used by race physics."""
from concurrent.futures import ThreadPoolExecutor
import gzip
import hashlib
import json
import os
from pathlib import Path
import random
import subprocess

from replay_protocol import decode, read_wire


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


class Worker:
    """One request at a time, with a deadline covering both write and read.

    On timeout or malformed/truncated output the process is killed and reaped.
    A later request starts a fresh worker, so one crash cannot poison the panel.
    """
    def __init__(self, command, log, *, timeout=60, output='binary-v1', replay_worker=False):
        self.command, self.log, self.timeout = command, log, timeout
        self.output, self.process = output, None
        self.env = {k: v for k, v in os.environ.items() if not k.startswith('UMA_')}
        self.env.update(GLOBALSIM_OUTPUT=output, GLOBALSIM_REPLAY_WORKER=str(int(replay_worker)))
        self.pool = ThreadPoolExecutor(max_workers=1)

    def _stop(self):
        if self.process is not None:
            if self.process.poll() is None:
                self.process.kill()
            self.process.wait(timeout=10)
            for pipe in (self.process.stdin, self.process.stdout):
                try:
                    pipe.close()
                except OSError:
                    pass  # A broken pipe is already reported by exchange/exit.
            self.process = None

    def _exchange(self, wire):
        self.process.stdin.write(wire)
        self.process.stdin.flush()
        if self.output == 'binary-v1':
            reply = read_wire(self.process.stdout)
            decode(reply)  # Reject malformed metadata before reusing the process.
        else:
            reply = self.process.stdout.readline(128 * 1024 * 1024 + 1)
            if not reply.endswith(b'\n') or len(reply) > 128 * 1024 * 1024:
                raise ValueError('Truncated or oversized JSON reply')
            json.loads(reply)
        return reply

    def exchange(self, wire):
        if self.process is None:
            self.process = subprocess.Popen(self.command, stdin=subprocess.PIPE,
                stdout=subprocess.PIPE, stderr=self.log, env=self.env)
        future = self.pool.submit(self._exchange, wire)
        try:
            return future.result(timeout=self.timeout)
        except Exception:
            # Kill before joining the read thread, but keep its process handle
            # alive until it has finished using the pipes.
            if self.process.poll() is None:
                self.process.kill()
            self.process.wait(timeout=10)
            try:
                future.result(timeout=10)
            except Exception:
                pass
            self._stop()
            raise

    def __enter__(self):
        return self

    def __exit__(self, *args):
        try:
            if self.process is not None:
                try:
                    self.process.stdin.close()
                except OSError:
                    pass
                code = self.process.wait(timeout=self.timeout)
                if code and not args[0]:
                    raise RuntimeError(f'Validation worker exited with code {code}')
        finally:
            self._stop()
            self.pool.shutdown(wait=True)


def save_case(directory, *, row, wire=None, expected=None, reply=None):
    """Self-contained private evidence, including complete skill definitions."""
    directory = Path(directory)
    directory.mkdir(parents=True, exist_ok=True)
    artifacts = {}
    for name, raw in [('input.json.gz', wire),
                      ('expected.json.gz', None if expected is None else json.dumps(expected).encode()),
                      ('actual.bin.gz', reply)]:
        if raw is not None:
            packed = gzip.compress(raw, compresslevel=1, mtime=0)
            (directory / name).write_bytes(packed)
            artifacts[name] = dict(sha256=digest(packed), contentSha256=digest(raw))
    (directory / 'audit.json').write_text(json.dumps(dict(row, artifacts=artifacts), indent=2) + '\n', encoding='utf-8')


def repeatability(engine, samples, output, *, rounds=3, timeout=60):
    """Check fresh processes, changed batch order, duplicate inputs and JSON modes.

    samples contain key, wire and the original dense binary reply. All inputs
    and baselines are saved so a sporadic failure remains reproducible evidence.
    Repeated agreement is a probe, not proof of determinism for all inputs.
    """
    if rounds < 1 or not samples:
        raise ValueError('Repeatability needs samples and at least one fresh round')
    output = Path(output)
    output.mkdir(parents=True, exist_ok=False)
    for i, sample in enumerate(samples):
        save_case(output / f'case-{i:03d}', row=dict(key=sample['key']),
                  wire=sample['wire'], reply=sample['reply'])
    rows = []
    modes = [('binary-v1', False)] * rounds + [('json', False), ('json', True)]
    for round_index, (mode, worker_mode) in enumerate(modes):
        order = list(range(len(samples)))
        if round_index == 0:
            order.reverse()
        else:
            random.Random(round_index).shuffle(order)
        order.append(order[0])  # Revisit an input after processing another batch.
        with (output / f'process-{round_index}.log').open('wb') as log, Worker(
                [str(Path(engine).resolve())], log, timeout=timeout,
                output=mode, replay_worker=worker_mode) as worker:
            for position, index in enumerate(order):
                sample = samples[index]
                row = dict(round=round_index, position=position, sample=index, key=sample['key'],
                           mode=mode, replayWorker=worker_mode, inputSha256=digest(sample['wire']))
                reply = None
                try:
                    reply = worker.exchange(sample['wire'])
                    row['outputSha256'] = digest(reply)
                    if mode == 'binary-v1':
                        if reply != sample['reply']:
                            raise AssertionError('Binary output differs across process or batch order')
                    else:
                        decode(sample['reply']).assert_matches(json.loads(reply))
                    row['passed'] = True
                except Exception as error:
                    row.update(passed=False, error=f'{type(error).__name__}: {error}')
                    failure = output / f'failure-{round_index}-{position}'
                    save_case(failure, row=row, wire=sample['wire'])
                    if reply is not None:
                        (failure / ('actual.bin' if mode == 'binary-v1' else 'actual.json')).write_bytes(reply)
                rows.append(row)
    result = dict(passed=all(r['passed'] for r in rows), samples=len(samples),
                  freshBinaryProcesses=rounds, jsonModes=2, checks=len(rows), cases=rows,
                  limitation='Finite process/order probes do not prove universal determinism or official accuracy.')
    (output / 'report.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
    return result
