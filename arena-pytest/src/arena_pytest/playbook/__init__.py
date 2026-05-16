from __future__ import annotations

import functools
import inspect
from contextlib import ExitStack, contextmanager
from typing import Any, Callable, Iterator


@contextmanager
def active_playbooks(arena: Any, *playbooks: Any) -> Iterator[None]:
    if not playbooks:
        raise ValueError("active_playbooks requires at least one playbook argument")

    for pb in playbooks:
        if not hasattr(pb, "run"):
            raise TypeError(
                f"{pb!r} is not a registered playbook "
                "(missing .run(arena) method)"
            )

    with ExitStack() as stack:
        for pb in playbooks:
            stack.enter_context(pb.run(arena))
        yield


def playbook(*playbooks: Any) -> Callable[[Callable], Callable]:
    if not playbooks:
        raise ValueError("@playbook requires at least one playbook argument")

    for pb in playbooks:
        if not hasattr(pb, "run"):
            raise TypeError(
                f"{pb!r} is not a registered playbook "
                "(missing .run(arena) method)"
            )

    def decorator(test_fn: Callable) -> Callable:
        is_coro = inspect.iscoroutinefunction(test_fn)

        if is_coro:

            @functools.wraps(test_fn)
            async def async_wrapper(*args, **kwargs):
                arena = _resolve_arena(test_fn, args, kwargs)
                with active_playbooks(arena, *playbooks):
                    return await test_fn(*args, **kwargs)

            return async_wrapper

        @functools.wraps(test_fn)
        def sync_wrapper(*args, **kwargs):
            arena = _resolve_arena(test_fn, args, kwargs)
            with active_playbooks(arena, *playbooks):
                return test_fn(*args, **kwargs)

        return sync_wrapper

    return decorator


def _resolve_arena(fn: Callable, args: tuple, kwargs: dict):
    if "arena" in kwargs:
        return kwargs["arena"]

    try:
        params = list(inspect.signature(fn).parameters.keys())
    except (TypeError, ValueError):
        raise RuntimeError(
            "@playbook could not resolve the 'arena' fixture for this test"
        )
    if "arena" not in params:
        raise RuntimeError(
            "tests decorated with @playbook must take the 'arena' fixture "
            "as a parameter"
        )
    idx = params.index("arena")
    if idx >= len(args):
        raise RuntimeError(
            "@playbook could not resolve 'arena' fixture positionally"
        )
    return args[idx]
