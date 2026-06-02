# arena-pytest

Pytest library for Arena (C FFI). Dependencies are managed with Bazel (`@pip//`); see the repo root `MODULE.bazel`.

## Example

Stand up a match with real dependencies, then use an HTTP playbook to script scenarios against a running dependency. Mappings are scoped to the `with` block and `expect_called` is verified automatically on exit.

```python
import requests
from arena_pytest import (
    ClosedArena,
    ExecutableComponentBuilder,
    HttpDependencyBuilder,
    HttpPlaybookBuilder,
    HttpReadinessCheck,
    KafkaDependencyBuilder,
    KafkaFlavor,
    MatchBuilder,
    PostgresDependencyBuilder,
)

# --- arena setup (usually lives in conftest.py) ---

postgres = PostgresDependencyBuilder("readings").with_port(5432).with_database_name("mydb").build()
kafka = KafkaDependencyBuilder("readings").with_flavor(KafkaFlavor.APACHE_NATIVE).with_port(9092).with_topic("events").build()
calibration = HttpDependencyBuilder("calibration").with_port(3003).build()

web_app = (
    ExecutableComponentBuilder("my service")
    .with_executable_path("/path/to/your/binary")
    .with_readiness_check(HttpReadinessCheck(), "http://127.0.0.1:8080/health")
    .build()
)

a_match = (
    MatchBuilder("my-test")
    .add_dependency(postgres)
    .add_dependency(kafka)
    .add_dependency(calibration)
    .add_component(web_app)
    .build()
)

closed = ClosedArena("Arena", [a_match])

# --- in a test ---

async def test_calibration_outage_returns_500():
    arena = await closed.open()
    try:
        outage = (
            HttpPlaybookBuilder(calibration.identifier)
            .with_mapping(
                method="POST",
                url_path="/api/v1/validate",
                status=500,
                priority=1,
                expect_called=1,
            )
            .build(arena)
        )

        with outage:
            r = requests.post("http://127.0.0.1:8080/readings", json={"value": 42})
            assert r.status_code == 500
        # outage context exits: mapping removed, expectation verified.
    finally:
        await arena.close()
```

## Support matrix

**Python:** 3.9+

Wheels ship a **native** FFI shared library next to the `arena_pytest` package (loaded via ctypes). Install a wheel whose **platform tag** matches your OS and CPU; there is no supported pure-Python fallback.

| Platform | Produced by CI ([build-test-publish-arena.yml](https://github.com/daveepope/arena/blob/master/.github/workflows/build-test-publish-arena.yml): wheels in `build_and_test`, upload in `publish_testpypi` on PRs) | Typical wheel tag |
|----------|---------------------------------------------------------------------------------------------|-------------------|
| Linux x86_64 | Yes (`ubuntu-latest`) | `manylinux2014_x86_64` |
| macOS arm64 | Yes (`macos-latest`) | `macosx_*_arm64` |
| Linux aarch64 | No — build locally | `manylinux2014_aarch64` |
| macOS x86_64 | No — build on Intel Mac | `macosx_*_x86_64` |
| Windows x86_64 | No — build on Windows | `win_amd64` |

For each release, upload **all** wheels you intend to support; PyPI stores them side by side and `pip` selects the correct one.

## Build the installable wheel (for PyPI or local `pip install`)

From the repository root:

```bash
bazel build //arena-pytest:arena_pytest_wheel
```

The wheel is under `bazel-bin/arena-pytest/`. The filename is **platform-specific** (it includes the Rust FFI shared library next to `arena_pytest/`), e.g. `arena_pytest-1.0.0-py3-none-manylinux2014_x86_64.whl` on Linux x86_64. Build on each OS/arch you want to publish and upload **each** wheel to PyPI.

Try it in another project:

```bash
pip install /path/to/repo/bazel-bin/arena-pytest/arena_pytest-*-manylinux*.whl
```

If `pip` sees two wheels for the same version, remove old files under `bazel-bin/arena-pytest/` and rebuild.

`pyproject.toml` stays available for setuptools-based workflows (e.g. `python -m build` in a normal Python environment), but **this repo’s canonical build for the wheel is Bazel**, matching `//arena-pytest:arena_pytest_wheel`.

### Tests vs wheel (same code, Bazel-native)

`//arena-pytest:arena_pytest_wheel` packages `:arena_pytest_lib`—the same Python sources tests use—**plus** the `arena_ffi_shared` native library for that host OS so `pip install` can load the core framework via ctypes without Bazel runfiles.

## Publish to PyPI

Each PR to `master` gets an automatic release bump from `master`’s `VERSION` (default **patch**). For a larger bump, add PR label **`semver:minor`** or **`semver:major`** (CI creates those labels on first run), or put **`[semver:minor]`** / **`[semver:major]`** in the PR title. CI commits `VERSION`, `Cargo.toml`, and `MODULE.bazel`. Keep `arena-pytest/LICENSE` aligned with the repository root `LICENSE`.

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
