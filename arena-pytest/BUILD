load("@bazel_skylib//rules:copy_file.bzl", "copy_file")
load("@rules_python//python:defs.bzl", "py_library", "py_test")
load("@rules_python//python:packaging.bzl", "py_wheel")
load("@rules_python//python:pip.bzl", "compile_pip_requirements")

ARENA_PYTEST_VERSION = "0.4.0b1"

# Keep in sync with classifiers in arena-pytest/pyproject.toml (Bazel wheel METADATA).
ARENA_PYTEST_CLASSIFIERS = [
    "Development Status :: 4 - Beta",
    "Framework :: Pytest",
    "Intended Audience :: Developers",
    "License :: OSI Approved :: MIT License",
    "Operating System :: MacOS :: MacOS X",
    "Operating System :: Microsoft :: Windows",
    "Operating System :: POSIX :: Linux",
    "Programming Language :: Python :: 3",
    "Programming Language :: Python :: 3.9",
    "Programming Language :: Python :: 3.10",
    "Programming Language :: Python :: 3.11",
    "Programming Language :: Python :: 3.12",
    "Programming Language :: Python :: 3.13",
    "Topic :: Software Development :: Testing",
]

exports_files(
    ["README.md", "LICENSE", "requirements.txt", "requirements_lock.txt"],
    visibility = ["//visibility:public"],
)

compile_pip_requirements(
    name = "pip_requirements",
    src = "requirements.txt",
    requirements_txt = "requirements_lock.txt",
    generate_hashes = False,
)

config_setting(
    name = "linux_x86_64",
    constraint_values = [
        "@platforms//os:linux",
        "@platforms//cpu:x86_64",
    ],
)

config_setting(
    name = "linux_aarch64",
    constraint_values = [
        "@platforms//os:linux",
        "@platforms//cpu:aarch64",
    ],
)

config_setting(
    name = "macos_aarch64",
    constraint_values = [
        "@platforms//os:macos",
        "@platforms//cpu:aarch64",
    ],
)

config_setting(
    name = "macos_x86_64",
    constraint_values = [
        "@platforms//os:macos",
        "@platforms//cpu:x86_64",
    ],
)

copy_file(
    name = "arena_ffi_shared_for_wheel_linux",
    src = "//arena-ffi:arena_ffi_shared",
    out = "src/arena_pytest/libarena_ffi_shared.so",
)

copy_file(
    name = "arena_ffi_shared_for_wheel_macos",
    src = "//arena-ffi:arena_ffi_shared",
    out = "src/arena_pytest/libarena_ffi_shared.dylib",
)

copy_file(
    name = "arena_ffi_shared_for_wheel_windows",
    src = "//arena-ffi:arena_ffi_shared",
    out = "src/arena_pytest/arena_ffi_shared.dll",
)

alias(
    name = "arena_ffi_shared_for_wheel",
    actual = select({
        "@platforms//os:linux": ":arena_ffi_shared_for_wheel_linux",
        "@platforms//os:macos": ":arena_ffi_shared_for_wheel_macos",
        "@platforms//os:windows": ":arena_ffi_shared_for_wheel_windows",
        "//conditions:default": ":arena_ffi_shared_for_wheel_linux",
    }),
)

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
    size = "small",
    srcs = [
        "tests/conftest.py",
        "tests/test_arena.py",
    ],
    main = "tests/test_arena.py",
    data = [
        "//arena-ffi:arena_ffi_shared",
        "//examples:instrument_reading_db_schema",
        "//examples:oauth_tls_resources",
        "//examples:validation_db_schema",
        "//examples:example-readings-axum-web-app",
    ],
    deps = [
        ":arena_pytest_lib",
        "@pip//kafka_python_ng",
        "@pip//pytest",
        "@pip//pytest_asyncio",
        "@pip//requests",
    ],
    tags = ["requires_docker"],
    python_version = "PY3",
    visibility = ["//visibility:public"],
)

py_wheel(
    name = "arena_pytest_wheel",
    distribution = "arena-pytest",
    version = ARENA_PYTEST_VERSION,
    python_tag = "py3",
    python_requires = ">=3.9",
    abi = "none",
    platform = select({
        ":linux_x86_64": "manylinux2014_x86_64",
        ":linux_aarch64": "manylinux2014_aarch64",
        ":macos_aarch64": "macosx_11_0_arm64",
        ":macos_x86_64": "macosx_10_12_x86_64",
        "@platforms//os:windows": "win_amd64",
        "//conditions:default": "any",
    }),
    strip_path_prefixes = ["arena-pytest/src"],
    author = "David Pope",
    classifiers = ARENA_PYTEST_CLASSIFIERS,
    description_file = "DESCRIPTION.md",
    description_content_type = "text/markdown",
    homepage = "https://github.com/daveepope/arena",
    license = "MIT",
    project_urls = {
        "Repository": "https://github.com/daveepope/arena",
        "Issues": "https://github.com/daveepope/arena/issues",
        "Changelog": "https://github.com/daveepope/arena/releases",
    },
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
    deps = [
        ":arena_pytest_lib",
        ":arena_ffi_shared_for_wheel",
    ],
    visibility = ["//visibility:public"],
)
