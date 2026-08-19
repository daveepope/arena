import sys
import zipfile
from pathlib import Path


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: nuget_native_libs_layout_test.py <nupkg-path>", file=sys.stderr)
        return 1

    nupkg_path = Path(sys.argv[1])
    with zipfile.ZipFile(nupkg_path) as zf:
        names = set(zf.namelist())

    expected = {
        "runtimes/osx-arm64/native/fake_native_osx_arm64.txt",
        "runtimes/osx-x64/native/fake_native_osx_x64.txt",
        "runtimes/linux-x64/native/fake_native_linux_x64.txt",
        "runtimes/win-x64/native/fake_native_win_x64.txt",
    }
    missing = expected - names
    if missing:
        print(f"FAILED: missing expected runtimes/<rid>/native/ entries: {missing}", file=sys.stderr)
        print(f"actual entries: {sorted(names)}", file=sys.stderr)
        return 1

    leaked_flat = {n for n in names if n.startswith("lib/") and n.endswith(".txt")}
    if leaked_flat:
        print(f"FAILED: native libs leaked into lib/<tfm>/ instead of runtimes/<rid>/native/: {leaked_flat}", file=sys.stderr)
        return 1

    print("PASSED: native libs are packaged under runtimes/<rid>/native/, not lib/<tfm>/")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
