"""The fast dense checker must produce the same report as the full checker."""
import copy
import json
from pathlib import Path
import struct
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
from check_accuracy_regressions import check_strict
from check_binary_accuracy import check_binary
from replay_protocol import decode, FIELDS
from test_accuracy_regressions import example


def packet(actual):
    meta = json.dumps({k:v for k,v in actual.items() if k != 'frames'}).encode()
    frames = actual['frames']
    n = len(frames[0]['horses'])
    payload = b''.join(struct.pack('<'+'d'*(1+6*n), frame['t'],
        *(h[k] for h in frame['horses'] for k in FIELDS)) for frame in frames)
    return decode(struct.pack('<8sQIIQ', b'GSIMRP01', len(meta), len(frames), n, len(payload)) + meta + payload)


class BinaryAuditTests(unittest.TestCase):
    def compare(self, expected, actual):
        replay = packet(actual)
        self.assertEqual(check_binary(expected, replay),
                         check_strict(expected, replay.to_json_value(), dense_frames=True))

    def test_dense_unsampled_mutations_and_clean_frame(self):
        for key, value in [('speed', 4.), ('hp', float('nan')), ('lane', float('inf')),
                           ('blockFrontHorseIndex', 0), ('blockFrontHorseIndex', .5),
                           ('temptationMode', -1), ('temptationMode', .5)]:
            expected, actual = example()
            hidden = copy.deepcopy(actual['frames'][0])
            hidden['t'] = .5
            hidden['horses'][0][key] = value
            actual['frames'].insert(1, hidden)
            with self.subTest(key=key, value=value):
                self.compare(expected, actual)

    def test_timestamps_missing_observations_and_results(self):
        for stamp in [0., 1e100, float('nan'), -1., .000001]:
            expected, actual = example()
            hidden = copy.deepcopy(actual['frames'][0])
            hidden['t'] = stamp
            actual['frames'].insert(1, hidden)
            self.compare(expected, actual)
        expected, actual = example()
        expected['frames'].append([3., copy.deepcopy(expected['frames'][-1][1])])
        self.compare(expected, actual)
        expected, actual = example()
        actual['results'][0]['finishTimeRaw'] = 2.
        self.compare(expected, actual)

    def test_legacy_timestamp_collision_cannot_be_dropped(self):
        expected, actual = example()
        hidden = copy.deepcopy(actual['frames'][0])
        hidden['t'] = .000001
        hidden['horses'][0]['pos'] = 100.
        actual['frames'].insert(1, hidden)
        # Exact float32 samples still match, but the legacy rounded lookup
        # selects the later frame. Both validators must retain that failure.
        self.compare(expected, actual)
        self.assertIn('legacyChecks', check_binary(expected, packet(actual))['differences'])


if __name__ == '__main__':
    unittest.main()
