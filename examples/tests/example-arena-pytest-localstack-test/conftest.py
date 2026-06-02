def _arena_plugin_already_registered_via_entry_point() -> bool:
    try:
        from importlib.metadata import entry_points

        eps = entry_points(group="pytest11")
    except Exception:
        return False
    return any(getattr(ep, "value", "") == "arena_pytest.arena" for ep in eps)


if not _arena_plugin_already_registered_via_entry_point():
    pytest_plugins = ("arena_pytest.arena",)
