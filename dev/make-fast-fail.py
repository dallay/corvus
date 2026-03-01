#!/usr/bin/env python3

import subprocess
import sys
import time


def main() -> int:
    if len(sys.argv) < 3:
        print(
            "Usage: make-fast-fail.py <make-bin> <target> [<target> ...]",
            file=sys.stderr,
        )
        return 2

    make_bin = sys.argv[1]
    targets = sys.argv[2:]

    alive = {}
    for target in targets:
        cmd = [make_bin, "--no-print-directory", "CHECK_TOOLS=0", target]
        proc = subprocess.Popen(cmd)
        alive[proc] = target
        print(f"🚀 Started {target}", flush=True)

    while alive:
        time.sleep(0.2)
        for proc, target in list(alive.items()):
            code = proc.poll()
            if code is None:
                continue

            if code == 0:
                print(f"✅ Completed {target}", flush=True)
                del alive[proc]
                continue

            print(f"❌ Failed: {target}", flush=True)
            for other in list(alive.keys()):
                if other is proc:
                    continue
                try:
                    other.terminate()
                except Exception:
                    pass

            deadline = time.time() + 5
            for other in list(alive.keys()):
                if other is proc:
                    continue
                while other.poll() is None and time.time() < deadline:
                    time.sleep(0.1)
                if other.poll() is None:
                    try:
                        other.kill()
                    except Exception:
                        pass

            return code

    print("✨ Fast-fail pipeline completed successfully!", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
