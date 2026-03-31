load("@rules_python//python:defs.bzl", "py_library", "py_test")
load("@rules_python//python:packaging.bzl", "py_package", "py_wheel")

py_library(
    name = "arena_pytest_lib",
    srcs = glob(["src/arena_pytest/**/*.py"]),
    imports = ["src"],
    deps = [
        "@pip//kafka_python_ng",
        "@pip//pytest",
        "@pip//pytest_asyncio",
        "@pip//requests",
    ],
    visibility = ["//visibility:public"],
)

py_test(
    name = "arena_pytest_test",
    srcs = [
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
        ":arena_pytest_lib",
        "@pip//kafka_python_ng",
        "@pip//pytest",
        "@pip//pytest_asyncio",
        "@pip//requests",
    ],
    tags = ["local"],
    python_version = "PY3",
    visibility = ["//visibility:public"],
)

py_package(
    name = "arena_pytest_pkg",
    packages = ["arena_pytest"],
    deps = [":arena_pytest_lib"],
)

py_wheel(
    name = "arena_pytest_wheel",
    distribution = "arena-pytest",
    version = "0.1.0-alpha.1",
    python_tag = "py3",
    python_requires = ">=3.9",
    summary = "pytest API and fixtures for Arena (via the C FFI)",
    requires = [
        "pytest>=7.0.0",
        "pytest-asyncio>=0.21.0",
        "requests>=2.28.0",
        "kafka-python-ng>=2.0.0",
    ],
    entry_points = {
        "pytest11": ["arena_pytest = arena_pytest.arena"],
    },
    deps = [":arena_pytest_pkg"],
    visibility = ["//visibility:public"],
)
