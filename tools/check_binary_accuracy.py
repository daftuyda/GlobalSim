"""Dense binary validation with a vectorized integrity pass and canonical fallback.

Every dense scalar and timestamp is checked. Only conversion to Python frame
dictionaries is restricted to frames used by the recorded-observation checks.
Any failure uses the original full checker for identical diagnostics.
"""
import numpy as np
from check_accuracy_regressions import check_strict, f32
from replay_protocol import FIELDS


def check_binary(expected, packet, *, spec=None):
    def full():
        return check_strict(expected, packet.to_json_value(), dense_frames=True, spec=spec)
    n = len(expected.get('finish', []))
    if 'error' in packet.metadata or packet.horse_count != n or not packet.count or not n:
        return full()
    dense = np.frombuffer(packet.frames, dtype='<f8').reshape(packet.count, 1 + 6 * n)
    if not np.isfinite(dense).all():
        return full()
    state = dense[:, 1:].reshape(packet.count, n, 6)
    mode, blocker = state[:, :, 4], state[:, :, 5]
    if not ((mode == np.floor(mode)) & (mode >= 0) & (mode <= 127)).all():
        return full()
    if not ((blocker == np.floor(blocker)) & (blocker >= -1) & (blocker < n)
            & (blocker != np.arange(n))).all():
        return full()
    with np.errstate(over='ignore'):
        stamps = dense[:, 0].astype(np.float32)
    if not np.isfinite(stamps).all() or not (stamps[1:] > stamps[:-1]).all():
        return full()
    try:
        wanted = {f32(t) for t, _ in expected['frames']}
        rounded = {round(t, 5) for t, _ in expected['frames']}
    except (KeyError, TypeError, ValueError, OverflowError):
        return full()
    frames = []
    for index, (raw_t, stamp) in enumerate(zip(dense[:, 0], stamps)):
        t = float(raw_t)
        # Preserve both lookups, including collisions and last-wins behavior in
        # the legacy comparison; use Python round, not NumPy's different rounding.
        if float(stamp) in wanted or round(t, 5) in rounded:
            frames.append(dict(t=t, horses=[dict(zip(FIELDS, map(float, values))) for values in state[index]]))
    result = check_strict(expected, dict(packet.metadata, frames=frames), dense_frames=True, spec=spec)
    if not result['passed']:
        return full()
    result['counts']['checkedSimulationFrames'] = packet.count
    result['counts']['checkedSimulationRunnerSamples'] = packet.count * n
    return result
