import atexit
import collections
import os
import signal
import stat
import sys
import threading
import time

_INTERVAL_SECONDS = __ARENA_INTERVAL_SECONDS__
_FIRST_FLUSH_SECONDS = __ARENA_FIRST_FLUSH_SECONDS__
_FLUSH_CEILING_SECONDS = __ARENA_FLUSH_CEILING_SECONDS__
_MAX_DISTINCT_STACKS = __ARENA_MAX_DISTINCT_STACKS__
_MAX_BYTES = __ARENA_MAX_BYTES__
_REQUIRED_VERSION = (3, 11)
_PRIVATE_MODE = 0o700

_SELF = os.path.abspath(__file__)
_DIRECTORY = os.path.dirname(_SELF)
_OUTPUT = os.path.join(_DIRECTORY, "profile-%d.folded" % os.getpid())
_STATUS = os.path.join(_DIRECTORY, "status-%d.txt" % os.getpid())

_counts = collections.Counter()
_flushes = 0
_stopped = threading.Event()
_writing = threading.Lock()
_flusher_thread = None


def _refuse(reason):
    try:
        descriptor = os.open(
            _STATUS, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600
        )
    except OSError:
        _discard()
        return
    try:
        with os.fdopen(descriptor, "w") as status:
            status.write("refused %s\n" % reason)
    except OSError:
        _discard()


def _discard():
    try:
        os.unlink(_OUTPUT)
    except OSError:
        pass


def _disarm():
    try:
        signal.setitimer(signal.ITIMER_PROF, 0)
    except (OSError, ValueError):
        pass
    _stopped.set()


def _stop(reason):
    _disarm()
    _refuse(reason)


def _directory_is_private():
    try:
        held = os.stat(_DIRECTORY)
    except OSError:
        return False
    return held.st_uid == os.geteuid() and stat.S_IMODE(held.st_mode) == _PRIVATE_MODE


def _collapse(frame):
    stack = []
    while frame is not None:
        code = frame.f_code
        if code.co_filename != _SELF:
            stack.append("%s (%s:%d)" % (code.co_qualname, code.co_filename, frame.f_lineno))
        frame = frame.f_back
    stack.reverse()
    return ";".join(stack)


def _rendered(items):
    return "".join("%s %d\n" % (stack, count) for stack, count in items if stack)


def _write(body):
    global _flushes
    _flushes += 1
    pending = os.path.join(_DIRECTORY, "profile-%d.%d.pending" % (os.getpid(), _flushes))
    descriptor = os.open(
        pending, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600
    )
    with os.fdopen(descriptor, "wb") as out:
        out.write(body)
    os.replace(pending, _OUTPUT)


def _flush():
    with _writing:
        _flush_held()


def _flush_held():
    if signal.getsignal(signal.SIGPROF) is not _sample:
        _stop("another handler replaced SIGPROF while profiling")
        return
    if not _directory_is_private():
        _stop("the private profile directory changed owner or mode while profiling")
        return
    items = list(_counts.items())
    if len(items) > _MAX_DISTINCT_STACKS:
        _stop(
            "the profile reached the %d distinct stack limit; raise the cpu profile limit on the "
            "lifecycle dashboard builder" % _MAX_DISTINCT_STACKS
        )
        return
    body = _rendered(items).encode("utf-8")
    if len(body) > _MAX_BYTES:
        _stop(
            "the profile reached the %d byte limit; raise the cpu profile limit on the lifecycle "
            "dashboard builder" % _MAX_BYTES
        )
        return
    try:
        _write(body)
    except OSError as error:
        _stop("the profile could not be written: %s" % error)


def _sample(_signum, _frame):
    try:
        own = _flusher_thread
        for thread_id, frame in sys._current_frames().items():
            if thread_id == own:
                continue
            _counts[_collapse(frame)] += 1
    except Exception as error:
        _stop("the sampler could not record a stack: %s" % error)


def _abandon_after_fork():
    _counts.clear()
    _stopped.set()


def _flusher():
    wait_for = _FIRST_FLUSH_SECONDS
    while not _stopped.wait(wait_for):
        _flush()
        wait_for = min(wait_for * 2, _FLUSH_CEILING_SECONDS)


def _finalize():
    _disarm()
    if not _counts:
        return
    _flush()


def _arm():
    if sys.version_info < _REQUIRED_VERSION:
        _refuse(
            "this interpreter is %d.%d and the sampler needs %d.%d or newer"
            % (sys.version_info[0], sys.version_info[1], _REQUIRED_VERSION[0], _REQUIRED_VERSION[1])
        )
        return False
    if not _directory_is_private():
        _refuse("the profile directory is not private to this user at mode %o" % _PRIVATE_MODE)
        return False
    held = signal.getsignal(signal.SIGPROF)
    if held is not signal.SIG_DFL:
        _refuse("SIGPROF is already handled by %r in this process" % (held,))
        return False
    signal.signal(signal.SIGPROF, _sample)
    previous = signal.setitimer(signal.ITIMER_PROF, _INTERVAL_SECONDS, _INTERVAL_SECONDS)
    if previous != (0.0, 0.0):
        signal.signal(signal.SIGPROF, held)
        signal.setitimer(signal.ITIMER_PROF, previous[0], previous[1])
        _refuse("the process cpu timer was already armed as %r" % (previous,))
        return False
    return True


if threading.current_thread() is threading.main_thread():
    if _arm():
        _flusher_started = threading.Thread(
            target=_flusher, name="arena-cpu-profile-flush", daemon=True
        )
        _flusher_started.start()
        _flusher_thread = _flusher_started.ident
        os.register_at_fork(after_in_child=_abandon_after_fork)
        atexit.register(_finalize)
