load("@arena_version//:defs.bzl", "ARENA_VERSION")
load("@bazel_skylib//rules:copy_file.bzl", "copy_file")
load("//tools:version_sync.bzl", "sync_arena_version")
load("@rules_python//python:defs.bzl", "py_library", "py_test")
load("@rules_python//python:packaging.bzl", "py_wheel")
load("@rules_python//python:pip.bzl", "compile_pip_requirements")

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
    ["LICENSE", "requirements.txt", "requirements_lock.txt", "requirements_windows.txt", "pyproject.toml"],
    visibility = ["//visibility:public"],
)

compile_pip_requirements(
    name = "pip_requirements",
    src = "requirements.txt",
    requirements_txt = "requirements_lock.txt",
    requirements_windows = "requirements_windows.txt",
    generate_hashes = False,
    tags = ["no-windows"],
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
    src = "prebuilt/linux-x86-64/libarena_ffi_shared.so",
    out = "src/arena_pytest/libarena_ffi_shared.so",
    tags = ["manual"],
)

copy_file(
    name = "arena_ffi_shared_for_wheel_macos",
    src = select({
        ":macos_aarch64": "prebuilt/darwin-aarch64/libarena_ffi_shared.dylib",
        ":macos_x86_64": "prebuilt/darwin-x86-64/libarena_ffi_shared.dylib",
    }),
    out = "src/arena_pytest/libarena_ffi_shared.dylib",
    tags = ["manual"],
)

copy_file(
    name = "arena_ffi_shared_for_wheel_windows",
    src = "prebuilt/windows-x86-64/arena_ffi_shared.dll",
    out = "src/arena_pytest/arena_ffi_shared.dll",
    tags = ["manual"],
)

alias(
    name = "arena_ffi_shared_for_wheel",
    actual = select({
        "@platforms//os:linux": ":arena_ffi_shared_for_wheel_linux",
        "@platforms//os:macos": ":arena_ffi_shared_for_wheel_macos",
        "@platforms//os:windows": ":arena_ffi_shared_for_wheel_windows",
        "//conditions:default": ":arena_ffi_shared_for_wheel_linux",
    }),
    tags = ["manual"],
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
    name = "arena_pytest_unit_test",
    size = "small",
    srcs = glob(["tests/unit/*.py"]),
    main = "tests/unit/playbook_unit_test.py",
    data = ["//arena-ffi:arena_ffi_shared"],
    deps = [
        ":arena_pytest_lib",
        "@pip//pytest",
    ],
    python_version = "PY3",
    visibility = ["//visibility:public"],
)

py_test(
    name = "playbook_timing_component_test",
    size = "medium",
    srcs = glob(["tests/component/playbooks/*.py"]),
    main = "tests/component/playbooks/playbook_timing_component_test.py",
    imports = ["tests/component/playbooks"],
    data = ["//arena-ffi:arena_ffi_shared"],
    deps = [
        ":arena_pytest_lib",
        "@pip//pytest",
        "@pip//pytest_asyncio",
    ],
    tags = ["component_test"],
    python_version = "PY3",
    visibility = ["//visibility:public"],
)

py_test(
    name = "children_lifecycle_component_test",
    size = "small",
    srcs = glob(["tests/component/children/*.py"]),
    main = "tests/component/children/children_lifecycle_component_test.py",
    imports = ["tests/component/children"],
    data = ["//arena-ffi:arena_ffi_shared"],
    deps = [
        ":arena_pytest_lib",
        "@pip//pytest",
    ],
    tags = ["component_test"],
    python_version = "PY3",
    visibility = ["//visibility:public"],
)

sync_arena_version(
    name = "sync_version_before_wheel",
    stamp = "version_sync.stamp",
)

py_library(
    name = "arena_pytest_version_synced",
    data = [":sync_version_before_wheel"],
)

py_wheel(
    name = "arena_pytest_wheel",
    distribution = "arena-pytest",
    version = ARENA_VERSION,
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
    description_file = "//:README.md",
    description_content_type = "text/markdown",
    homepage = "https://github.com/daveepope/arena",
    license = "MIT",
    project_urls = {
        "Repository": "https://github.com/daveepope/arena",
        "Issues": "https://github.com/daveepope/arena/issues",
        "Changelog": "https://github.com/daveepope/arena/blob/master/CHANGELOG.md",
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
        ":arena_pytest_version_synced",
    ],
    tags = ["manual"],
    visibility = ["//visibility:public"],
)
