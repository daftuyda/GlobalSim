import io
import json
from pathlib import Path
import struct
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
from replay_protocol import decode, read_reply, read_wire, HEADER, MAGIC


class ReplayProtocolTests(unittest.TestCase):
    def test_partial_reads_multiple_packets_and_error_recovery(self):
        class Partial(io.BytesIO):
            def read(self, size=-1): return super().read(min(size, 7))
        meta = json.dumps({'seed': 7, 'name': 'é'}).encode()
        row = struct.pack('<7d', 1., 2., 3., 4., 5., 0., -1.)
        good = HEADER.pack(MAGIC, len(meta), 1, 1, len(row)) + meta + row
        error = b'{"error":"bad input"}'
        bad = HEADER.pack(MAGIC, len(error), 0, 0, 0) + error
        stream = Partial(good + bad + good)
        expected = {'seed': 7, 'name': 'é', 'frames': [{'t': 1., 'horses': [{
            'pos': 2., 'speed': 3., 'hp': 4., 'lane': 5., 'temptationMode': 0., 'blockFrontHorseIndex': -1.}]}]}
        reply = read_reply(stream)
        reply.assert_matches(expected)
        self.assertEqual(reply.to_json_value(), expected)
        self.assertTrue(reply.frames.readonly)
        self.assertEqual(read_reply(stream).to_json_value(), {'error': 'bad input'})
        read_reply(stream).assert_matches(expected)
        with self.assertRaises(EOFError): read_reply(stream)

    def test_rejects_lengths_versions_dimensions_and_truncation(self):
        for data in [b'bad', HEADER.pack(b'BADVERSN', 2, 0, 0, 0)+b'{}',
                     HEADER.pack(MAGIC, 2, 1, 1, 0)+b'{}',
                     HEADER.pack(MAGIC, 2, 0, 121, 0)+b'{}',
                     HEADER.pack(MAGIC, 3, 0, 0, 0)+b'{}',
                     HEADER.pack(MAGIC, 2, 0, 0, 0)+b'{}extra']:
            with self.assertRaises(ValueError): decode(data)
        with self.assertRaises(ValueError):
            read_wire(io.BytesIO(HEADER.pack(MAGIC, 999, 0, 0, 0)), max_bytes=100)
        with self.assertRaises(EOFError):
            read_reply(io.BytesIO(HEADER.pack(MAGIC, 2, 0, 0, 0)+b'{'))


if __name__ == '__main__': unittest.main()
