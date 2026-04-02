# arena-pytest

Pytest plugin and helpers for Arena (C FFI). Dependencies are managed with Bazel (`@pip//`); see the repo root `MODULE.bazel`.

## Build the installable wheel (for PyPI or local `pip install`)

From the repository root:

```bash
bazel build //arena-pytest:arena_pytest_wheel
```

The wheel is under `bazel-bin/arena-pytest/`. The filename is **platform-specific** (it includes the Rust FFI shared library next to `arena_pytest/`), e.g. `arena_pytest-0.1.0a1-py3-none-manylinux2014_x86_64.whl` on Linux x86_64. Build on each OS/arch you want to publish and upload **each** wheel to PyPI.

Try it in another project:

```bash
pip install /path/to/repo/bazel-bin/arena-pytest/arena_pytest-*-manylinux*.whl
```

If `pip` sees two wheels for the same version, remove old files under `bazel-bin/arena-pytest/` and rebuild.

`pyproject.toml` stays available for setuptools-based workflows (e.g. `python -m build` in a normal Python environment), but **this repo’s canonical build for the wheel is Bazel**, matching `//arena-pytest:arena_pytest_wheel`.

### Tests vs wheel (same code, Bazel-native)

`//arena-pytest:arena_pytest_wheel` packages `:arena_pytest_lib`—the same Python sources tests use—**plus** the `arena_ffi_shared` native library for that host OS so `pip install` can load the core framework via ctypes without Bazel runfiles.

## Publish to PyPI

Bump **`version`** in both `BUILD` (`py_wheel`) and `pyproject.toml` before a release, then rebuild the wheel.

### 1. Smoke test (install the wheel locally, no upload)

```bash
bazel build //arena-pytest:arena_pytest_wheel
python3 -m venv /tmp/arena-pytest-smoke
/tmp/arena-pytest-smoke/bin/pip install --upgrade pip
/tmp/arena-pytest-smoke/bin/pip install bazel-bin/arena-pytest/arena_pytest-*-manylinux*.whl
/tmp/arena-pytest-smoke/bin/python -c "import arena_pytest; print(arena_pytest.__file__)"
```

### 2. TestPyPI

Create a project and API token on [test.pypi.org](https://test.pypi.org). Install [twine](https://twine.readthedocs.io/) (`pip install twine`), then:

```bash
export TWINE_USERNAME=__token__
export TWINE_PASSWORD=<your-testpypi-token>

twine upload --repository testpypi bazel-bin/arena-pytest/arena_pytest-*-manylinux*.whl
```

Install from TestPyPI:

```bash
pip install --index-url https://test.pypi.org/simple/ --extra-index-url https://pypi.org/simple/ arena-pytest
```

### 3. Production PyPI

Create the `arena-pytest` project on [pypi.org](https://pypi.org) and an API token with upload scope for that project.

```bash
export TWINE_USERNAME=__token__
export TWINE_PASSWORD=<your-pypi-org-token>

twine upload bazel-bin/arena-pytest/arena_pytest-*-manylinux*.whl
```

After publishing, users install with `pip install arena-pytest`.

## Run tests in this repo

```bash
bazel test //arena-pytest:arena_pytest_test
```
