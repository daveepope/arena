from __future__ import annotations

import itertools
import os
import time

_SUFFIX_LEN = 6
_ALPHABET = "0123456789abcdefghijklmnopqrstuvwxyz"
_BASE = len(_ALPHABET)
_MASK_64 = 0xFFFFFFFFFFFFFFFF


def _seed() -> int:
    if not hasattr(_seed, "_value"):
        nanos = time.time_ns() & _MASK_64
        pid = os.getpid() & _MASK_64
        pid_rot = ((pid << 32) | (pid >> 32)) & _MASK_64
        _seed._value = nanos ^ pid_rot
    return _seed._value


_COUNTER = itertools.count()


def _slugify(s: str) -> str:
    out = []
    last_dash = False
    for c in s:
        c = c.lower()
        if c.isascii() and c.isalnum():
            out.append(c)
            last_dash = False
        elif not last_dash:
            out.append("-")
            last_dash = True
    return "".join(out).strip("-")


def _new_suffix() -> str:
    n = (_seed() + next(_COUNTER)) & _MASK_64
    digits = []
    for _ in range(_SUFFIX_LEN):
        digits.append(_ALPHABET[n % _BASE])
        n //= _BASE
    return "".join(reversed(digits))


def _has_suffix(name: str) -> bool:
    if "--" not in name:
        return False
    suffix = name.rsplit("--", 1)[-1]
    if len(suffix) != _SUFFIX_LEN:
        return False
    return all(c in _ALPHABET for c in suffix)


def build(module: str, name: str) -> str:
    if _has_suffix(name):
        return name
    slug = _slugify(name)
    suffix = _new_suffix()
    if not slug:
        return f"{module}-{suffix}"
    return f"{module}-{slug}--{suffix}"
