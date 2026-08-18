load("@rules_python//python:defs.bzl", "py_binary")

py_binary(
    name = "pypi_smoke_test",
    srcs = ["smoke_test_pypi.py"],
    main = "smoke_test_pypi.py",
    args = ["$(location //arena-pytest:arena_pytest_wheel)"],
    data = ["//arena-pytest:arena_pytest_wheel"],
    tags = ["manual"],
)
