"""Thread-confined GlobalSim prepared races through the C ABI (ctypes)."""
import ctypes
import json
from pathlib import Path
import threading
from replay_protocol import decode


class NativeRace:
    """Prepare once, run many seeds. Each run returns owned replay bytes.

    Create/use/close this object on one thread. Other threads may create their
    own handles. Frame views from returned Replay objects outlive later runs.
    """
    def __init__(self, library, spec):
        self._thread = threading.get_ident()
        self._handle = None
        self._lib = ctypes.CDLL(str(Path(library).resolve()))
        signatures = {
            'globalsim_new': (ctypes.c_void_p, []),
            'globalsim_prepare': (ctypes.c_int, [ctypes.c_void_p, ctypes.c_void_p, ctypes.c_size_t]),
            'globalsim_run': (ctypes.c_int, [ctypes.c_void_p, ctypes.c_int64]),
            'globalsim_data': (ctypes.c_void_p, [ctypes.c_void_p]),
            'globalsim_size': (ctypes.c_size_t, [ctypes.c_void_p]),
            'globalsim_error': (ctypes.c_char_p, [ctypes.c_void_p]),
            'globalsim_free': (ctypes.c_int, [ctypes.c_void_p]),
        }
        for name, (result, arguments) in signatures.items():
            function = getattr(self._lib, name)
            function.restype, function.argtypes = result, arguments
        self._handle = self._lib.globalsim_new()
        try:
            self.prepare(spec)
        except BaseException:
            self.close()
            raise

    def _check(self):
        if threading.get_ident() != self._thread:
            raise RuntimeError('Use a prepared race only on its creating thread')
        if self._handle is None:
            raise RuntimeError('Prepared race has been closed')

    def _success(self, status):
        if status:
            error = self._lib.globalsim_error(self._handle)
            raise ValueError(error.decode('utf-8') if error else f'GlobalSim status {status}')

    def prepare(self, spec):
        self._check()
        wire = spec if isinstance(spec, bytes) else json.dumps(spec, separators=(',', ':')).encode('utf-8')
        self._success(self._lib.globalsim_prepare(self._handle, wire, len(wire)))

    def run_wire(self, seed):
        self._check()
        if not isinstance(seed, int) or not -(1 << 63) <= seed < (1 << 63):
            raise ValueError('Seed must be a signed 64-bit integer')
        self._success(self._lib.globalsim_run(self._handle, seed))
        size = self._lib.globalsim_size(self._handle)
        pointer = self._lib.globalsim_data(self._handle)
        if not size or not pointer:
            raise RuntimeError('GlobalSim returned no packet')
        # Copy before a later run can reuse the native packet buffer.
        return ctypes.string_at(pointer, size)

    def run(self, seed):
        return decode(self.run_wire(seed))

    def close(self):
        if self._handle is not None:
            self._check()
            self._success(self._lib.globalsim_free(self._handle))
            self._handle = None

    def __enter__(self):
        self._check()
        return self

    def __exit__(self, *_):
        self.close()
