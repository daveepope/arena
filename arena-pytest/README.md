# arena-pytest

Pytest plugin and helpers for Arena (C FFI). Dependencies are managed with Bazel (`@pip//`); see the repo root `MODULE.bazel`.

## Build the installable wheel (for PyPI or local `pip install`)

From the repository root:

```bash
bazel build //arena-pytest:arena_pytest_wheel
```

The wheel is under `bazel-bin/arena-pytest/` (for example `arena_pytest-0.1.0a1-py3-none-any.whl`).

Try it in another project:

```bash
pip install /path/to/repo/bazel-bin/arena-pytest/arena_pytest-*.whl
```

`pyproject.toml` stays available for setuptools-based workflows (e.g. `python -m build` in a normal Python environment), but **this repo’s canonical build for the wheel is Bazel**, matching `//arena-pytest:arena_pytest_wheel`.

### Tests vs wheel (same code, Bazel-native)

`//arena-pytest:arena_pytest_wheel` is built from `:arena_pytest_pkg`, which collects the same `:arena_pytest_lib` sources used by `//arena-pytest:arena_pytest_test`. There is no separate `pip install` step inside tests: the wheel is the packaged form of that library graph, not a second implementation.

## Run tests in this repo

```bash
bazel test //arena-pytest:arena_pytest_test --test_tag_filters=local
```
