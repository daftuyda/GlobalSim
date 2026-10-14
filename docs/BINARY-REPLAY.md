# Binary replay interface

Set `GLOBALSIM_OUTPUT=binary-v1` when starting the engine to receive exact
numeric replay frames without JSON formatting for each runner observation.
Requests remain one JSON object per line. The default output is JSON.

Use `retainAllFrames: true` to return every simulation frame. With the default
`false`, the usual replay selection and finish samples apply. Both formats
preserve results, events, runner order, precision, and simulation behavior.
Binary output uses one simulation thread; `GLOBALSIM_REPLAY_WORKER` does not
start an encoding helper in this mode.

## Packet layout

Each request returns one packet, flushed when complete. Packets have no newline
terminator. Read the header, then exactly the advertised number of bytes.
All integers and floats use little-endian byte order.

| Offset | Bytes | Field |
|---|---:|---|
| 0 | 8 | ASCII magic `GSIMRP01` |
| 8 | 8 | Metadata length, unsigned 64-bit |
| 16 | 4 | Selected frame count, unsigned 32-bit |
| 20 | 4 | Runner count, unsigned 32-bit |
| 24 | 8 | Frame payload length, unsigned 64-bit |
| 32 | Metadata length | UTF-8 JSON reply with the `frames` field omitted |
| After metadata | Payload length | Frame records |

Every frame record contains its timestamp as one IEEE-754 float64, followed by
six float64 values for each runner in request order:

1. Position
2. Observed speed
3. HP
4. Lane
5. Rushing mode (`temptationMode` in JSON)
6. Front blocker index, or `-1`

Each record occupies `8 + 48 × runner_count` bytes. The payload length equals
that stride multiplied by the selected frame count. Numeric values keep their
original float64 bits, including values widened from float32. The encoding
does not use lossy quantization. Signed zero and nonfinite values retain their
bits; the JSON interface normalizes zero and writes nonfinite values as `null`.

Recoverable errors contain the usual `{"error":"..."}` metadata and zero frame
count, runner count, and payload length. The next request can still be served.
The same input-validation limits as the [JSON interface](INTERFACE.md) apply.

## Reading a reply

The standard-library [reader](../tools/replay_protocol.py) provides metadata
and a read-only frame buffer. It handles partial reads and checks packet sizes.

```python
import json
import os
import subprocess
from replay_protocol import read_reply  # Add GlobalSim/tools to the module path.

worker = subprocess.Popen(
    ["path/to/race_engine_rs_v2.exe"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    env=dict(os.environ, GLOBALSIM_OUTPUT="binary-v1"),
)
spec["retainAllFrames"] = True
worker.stdin.write((json.dumps(spec) + "\n").encode())
worker.stdin.flush()
replay = read_reply(worker.stdout)
if "error" in replay.metadata:
    raise RuntimeError(replay.metadata["error"])
time, runner_values = replay.frame(0)
# Keep the worker alive for subsequent requests.
worker.stdin.close()
worker.wait()
```

Applications using NumPy can view the payload directly without converting it
into thousands of Python objects:

```python
import numpy as np
rows = np.frombuffer(replay.frames, dtype="<f8").reshape(
    replay.count, 1 + 6 * replay.horse_count
)
times = rows[:, 0]
horses = rows[:, 1:].reshape(replay.count, replay.horse_count, 6)
```

`replay.to_json_value()` reconstructs the conventional object shape when needed.
That optional conversion has its own allocation and processing cost. Consumers
seeking minimum latency should use the numeric buffer directly.

## Measurement and verification

```powershell
python tools/benchmark_replay.py --engine target/pgo/race_engine_rs_v2.exe --baseline path/to/previous-engine.exe --inputs path/to/races.jsonl --repeats 5
python tools/check_replay_protocol.py --engine target/pgo/race_engine_rs_v2.exe --baseline path/to/previous-engine.exe --inputs path/to/races.jsonl --isolation
```

The benchmark includes request parsing, simulation, frame capture, encoding,
and pipe transfer. Caller encoding and decoding are excluded. All frames are
requested by default; add `--standard-frames` for the sparse replay mode.
Every distinct reply is compared outside the timed interval, including all
metadata and every frame value.
