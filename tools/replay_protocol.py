"""Read GlobalSim binary-v1 replies without materializing frame JSON objects.

Inputs remain JSON lines. Set GLOBALSIM_OUTPUT=binary-v1 on the engine process.
Replay.frames is a read-only byte view of fixed-size little-endian f64 records.
"""
from dataclasses import dataclass
import json
import math
import struct
from typing import BinaryIO

HEADER = struct.Struct('<8sQIIQ')
MAGIC = b'GSIMRP01'
FIELDS = ('pos', 'speed', 'hp', 'lane', 'temptationMode', 'blockFrontHorseIndex')


def read_exact(stream: BinaryIO, length: int) -> bytes:
    data = stream.read(length)
    if len(data) == length:
        return data
    chunks = [data]
    remaining = length - len(data)
    while remaining:
        data = stream.read(remaining)
        if not data:
            raise EOFError(f'Truncated GlobalSim reply: {remaining} bytes missing')
        chunks.append(data)
        remaining -= len(data)
    return b''.join(chunks)


def read_wire(stream: BinaryIO, *, max_bytes: int = 128 * 1024 * 1024) -> bytes:
    header = read_exact(stream, HEADER.size)
    magic, metadata, count, horses, payload = HEADER.unpack(header)
    if magic != MAGIC:
        raise ValueError('Unknown GlobalSim replay format')
    if payload != count * (8 + 48 * horses) or horses > 120:
        raise ValueError('Invalid GlobalSim frame dimensions')
    if metadata + payload > max_bytes:
        raise ValueError('GlobalSim reply exceeds the configured size limit')
    return header + read_exact(stream, metadata + payload)


@dataclass(frozen=True)
class Replay:
    metadata: dict
    count: int
    horse_count: int
    frames: memoryview

    @property
    def stride(self) -> int:
        return 8 + 48 * self.horse_count

    def frame(self, index: int) -> tuple:
        """Return (time, flat runner values), without any numeric conversion."""
        if not 0 <= index < self.count:
            raise IndexError(index)
        values = struct.unpack_from('<' + 'd' * (1 + 6 * self.horse_count),
                                    self.frames, index * self.stride)
        return values[0], values[1:]

    def to_json_value(self) -> dict:
        """Materialize the conventional reply shape when an application needs it.

        Nonfinite values follow the JSON serializer's null behavior. This is
        intentionally optional: numeric consumers can use the frame buffer directly.
        """
        if 'error' in self.metadata:
            return dict(self.metadata)
        frames = []
        finite = lambda n: n if math.isfinite(n) else None
        for index in range(self.count):
            t, row = self.frame(index)
            frames.append({'t': finite(t), 'horses': [
                dict(zip(FIELDS, map(finite, row[i:i + 6])))
                for i in range(0, len(row), 6)]})
        return dict(self.metadata, frames=frames)

    def assert_matches(self, reference: dict) -> None:
        """Compare every metadata field and frame scalar against a JSON reply."""
        expected_metadata = {k: v for k, v in reference.items() if k != 'frames'}
        if self.metadata != expected_metadata:
            raise AssertionError('Replay metadata differs')
        frames = reference.get('frames', [])
        if len(frames) != self.count:
            raise AssertionError('Replay frame count differs')
        pack = struct.Struct('<' + 'd' * (1 + 6 * self.horse_count))
        for i, frame in enumerate(frames):
            if len(frame['horses']) != self.horse_count:
                raise AssertionError(f'Runner count differs in frame {i}')
            values = [frame['t']]
            values.extend(h[f] for h in frame['horses'] for f in FIELDS)
            raw = self.frames[i * self.stride:(i + 1) * self.stride]
            if all(v is not None for v in values) and raw == pack.pack(*values):
                continue
            # JSON writes either sign of zero as 0 and nonfinite values as null.
            # All other finite scalars must retain the identical f64 bits.
            for expected, actual in zip(values, pack.unpack(raw)):
                if expected is None and not math.isfinite(actual):
                    continue
                if expected == 0 and actual == 0:
                    continue
                if expected is None or struct.pack('<d', expected) != struct.pack('<d', actual):
                    raise AssertionError(f'Numeric value differs in frame {i}: {expected} != {actual}')


def decode(wire: bytes) -> Replay:
    if len(wire) < HEADER.size:
        raise ValueError('Truncated GlobalSim header')
    magic, metadata_size, count, horses, payload = HEADER.unpack_from(wire)
    if magic != MAGIC or payload != count * (8 + 48 * horses) or horses > 120:
        raise ValueError('Invalid GlobalSim replay header')
    offset = HEADER.size + metadata_size
    if len(wire) != offset + payload:
        raise ValueError('GlobalSim reply length differs from its header')
    metadata = json.loads(wire[HEADER.size:offset])
    if not isinstance(metadata, dict) or 'frames' in metadata:
        raise ValueError('Invalid GlobalSim replay metadata')
    if 'error' in metadata and (count or horses or payload):
        raise ValueError('Error reply contains replay data')
    return Replay(metadata, count, horses, memoryview(wire)[offset:])


def read_reply(stream: BinaryIO, **kwargs) -> Replay:
    return decode(read_wire(stream, **kwargs))
