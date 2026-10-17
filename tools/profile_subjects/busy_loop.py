import pathlib
import sys
import time

STEP_SECONDS = 0.010

def foo1():
    end = time.monotonic() + STEP_SECONDS
    while time.monotonic() < end:
        pass

def foo2():
    end = time.monotonic() + STEP_SECONDS
    while time.monotonic() < end:
        pass

def foo3():
    end = time.monotonic() + STEP_SECONDS
    while time.monotonic() < end:
        pass

def foo4():
    end = time.monotonic() + STEP_SECONDS
    while time.monotonic() < end:
        pass

def foo5():
    end = time.monotonic() + STEP_SECONDS
    while time.monotonic() < end:
        pass

def foo6():
    end = time.monotonic() + STEP_SECONDS
    while time.monotonic() < end:
        pass

def foo7():
    end = time.monotonic() + STEP_SECONDS
    while time.monotonic() < end:
        pass

def foo8():
    end = time.monotonic() + STEP_SECONDS
    while time.monotonic() < end:
        pass

def foo9():
    end = time.monotonic() + STEP_SECONDS
    while time.monotonic() < end:
        pass

def bar(seconds):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        pass

def one_pass(total_seconds):
    foo1()
    foo2()
    foo3()
    foo4()
    foo5()
    foo6()
    foo7()
    foo8()
    foo9()
    bar(max(total_seconds - STEP_SECONDS * 9, STEP_SECONDS))

def main(argv):
    total_seconds = float(argv[1]) / 1000.0
    marker = pathlib.Path(argv[2]) if len(argv) > 2 else None
    while True:
        one_pass(total_seconds)
        if marker is None:
            return 0
        marker.write_bytes(b"1")


if __name__ == "__main__":
    sys.exit(main(sys.argv))
