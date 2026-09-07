import os
import signal
import subprocess
import sys
import tempfile
import textwrap

import pytest


def _session_source(teardown_marker: str, later_marker: str) -> str:
    return textwrap.dedent(
        f"""
        import os
        import time
        import pytest

        @pytest.fixture(scope="session")
        def containers():
            yield "started"
            with open({teardown_marker!r}, "w") as fh:
                fh.write("torn down")

        def test_a_long_running(containers):
            print("READY", flush=True)
            time.sleep(60)

        def test_b_runs_after(containers):
            with open({later_marker!r}, "w") as fh:
                fh.write("kept going")
        """
    )


def _run_until_ready(session_dir: str, source: str) -> subprocess.Popen:
    test_file = os.path.join(session_dir, "test_signalled_session.py")
    with open(test_file, "w") as fh:
        fh.write(source)

    process = subprocess.Popen(
        [
            sys.executable,
            "-m",
            "pytest",
            test_file,
            "-p",
            "arena_pytest.arena",
            "-p",
            "no:cacheprovider",
            "-s",
            "-q",
        ],
        cwd=session_dir,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        env={**os.environ, "PYTHONUNBUFFERED": "1"},
    )

    for line in process.stdout:
        if "READY" in line:
            return process
    raise AssertionError("session never reached the running test")


@pytest.mark.skipif(
    sys.platform == "win32",
    reason="posix signal delivery; windows terminates without running handlers",
)
@pytest.mark.parametrize("sent_signal", [signal.SIGTERM, signal.SIGINT])
def test_signalled_session_mid_test_runs_teardown_and_stops(sent_signal):
    with tempfile.TemporaryDirectory() as session_dir:
        teardown_marker = os.path.join(session_dir, "teardown.marker")
        later_marker = os.path.join(session_dir, "later.marker")
        process = _run_until_ready(
            session_dir, _session_source(teardown_marker, later_marker)
        )

        process.send_signal(sent_signal)
        try:
            output, _ = process.communicate(timeout=30)
        except subprocess.TimeoutExpired:
            process.kill()
            output, _ = process.communicate()
            raise AssertionError(
                f"session did not exit after {sent_signal!r}; output:\n{output}"
            )

        assert os.path.exists(teardown_marker), (
            f"session fixture teardown did not run after {sent_signal!r}; output:\n{output}"
        )
        assert not os.path.exists(later_marker), (
            f"session kept running tests after {sent_signal!r}; output:\n{output}"
        )
