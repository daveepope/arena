from __future__ import annotations

import itertools
import os
import time

_COUNTER = itertools.count()


def _is_hex(s: str) -> bool:
    try:
        int(s, 16)
        return True
    except ValueError:
        return False


def _has_guid_suffix(name: str) -> bool:
    last = name.rsplit(" ", 1)[-1] if " " in name else ""
    if len(last) != 36:
        return False
    parts = last.split("-")
    if [len(p) for p in parts] != [8, 4, 4, 4, 12]:
        return False
    return all(_is_hex(p) for p in parts)


def _new_guid() -> str:
    nanos = time.time_ns() & 0xFFFFFFFFFFFFFFFF
    pid = os.getpid() & 0xFFFF
    seq = next(_COUNTER) & 0x0000FFFFFFFFFFFF
    return (
        f"{(nanos >> 32) & 0xFFFFFFFF:08x}-"
        f"{(nanos >> 16) & 0xFFFF:04x}-"
        f"{nanos & 0xFFFF:04x}-"
        f"{pid:04x}-"
        f"{seq:012x}"
    )


def build(module: str, name: str) -> str:
    if _has_guid_suffix(name):
        return name
    name = name.strip()
    guid = _new_guid()
    if not name:
        return f"{module} - {guid}"
    return f"{module} - {name} - {guid}"
