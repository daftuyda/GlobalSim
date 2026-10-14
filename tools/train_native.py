"""Exercise prepared library paths when collecting compiler profiles."""
import argparse
import json
from pathlib import Path
from native_engine import NativeRace


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--library', type=Path, required=True)
    p.add_argument('--inputs', type=Path, action='append', required=True)
    args = p.parse_args()
    count = 0
    for path in args.inputs:
        for line in path.read_text(encoding='utf-8-sig').splitlines():
            if not line.strip(): continue
            spec = json.loads(line)
            with NativeRace(args.library, spec) as race:
                for _ in range(3): race.run_wire(spec['seed'])
            count += 1
    print(f'Profiled {count} prepared inputs and {3 * count} native races')


if __name__ == '__main__': main()
