from __future__ import annotations

from datetime import timedelta


def _expiry_seconds(expiry: timedelta) -> int:
    total = expiry.total_seconds()
    if total < 0:
        raise ValueError(f"expiry must not be negative: {expiry!r}")
    seconds = int(total)
    if seconds == 0 and total > 0:
        return 1
    return seconds
