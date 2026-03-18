load("@rules_python//python:defs.bzl", "py_test")

py_test(
    name = "arena_pytest_test",
    srcs = [
        "src/arena_pytest/__init__.py",
        "src/arena_pytest/arena.py",
        "src/arena_pytest/closed_arena.py",
        "src/arena_pytest/encounter.py",
        "src/arena_pytest/executable_component.py",
        "src/arena_pytest/_ffi.py",
        "src/arena_pytest/http_healthcheck.py",
        "src/arena_pytest/kafka.py",
        "src/arena_pytest/postgres.py",
        "tests/conftest.py",
        "tests/test_arena.py",
    ],
    main = "tests/test_arena.py",
    data = [
        "//arena-ffi:arena_ffi_shared",
        "//examples:instrument_reading_db_schema",
        "//examples:web-app",
    ],
    deps = [
        "@pip//kafka_python_ng",
        "@pip//pytest",
        "@pip//pytest_asyncio",
        "@pip//requests",
    ],
    tags = ["local"],
    python_version = "PY3",
    visibility = ["//visibility:public"],
)
