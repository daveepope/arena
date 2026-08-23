# Arena opt-level benchmarks

Times the lifecycle of one arena built from a published Arena client package version:
open it once (Postgres + HTTP dependencies, each with a managed playbook that
auto-runs on open), set up the DB and HTTP connections used for interaction once,
run N interact iterations against it, then close it once. Each iteration fires the
managed Postgres playbook's `verify` query and a real read/write round trip against a
`benchmark` table (created via a startup SQL script) that stores that iteration's own
timing, so the benchmark writes its own results into the database it's benchmarking.
The unmanaged playbook variants for Postgres and HTTP run once, right after open, to
prove the manual-invocation path works. Each tool pulls the requested version straight
from its real public registry (PyPI, Maven Central, NuGet), not a local Bazel build,
so these reach the network and aren't hermetic, hence `tags = ["manual"]` (excluded
from normal `bazel test //...` runs).

Each language's benchmark logic is written natively in that language: Python for
`pypi`, real Java for `maven`, real C# for `dotnet`. `maven` and `dotnet` are real
checked-in `pom.xml`/`.csproj` + `.java`/`.cs` project files, not generated code.
Postgres credentials are random per run, not hardcoded.

Reading/writing the `benchmark` table requires a real Postgres client driver in each
language (Arena's own playbook API is read-only verification, not arbitrary SQL):
`psycopg[binary]` (Python), `org.postgresql:postgresql` (Java JDBC), `Npgsql` (.NET).

Output goes straight to stdout, one line:
`version=<x> open_ms=<..> iterations=<n> interact_min_ms=<..> interact_ms=<median> interact_p95_ms=<..> interact_max_ms=<..> close_ms=<..> e2e_ms=<..>`

## pypi/ (`arena-pytest`)

```bash
bazel run //tools/benches/pypi:bench_pypi -- --version 6.1.0
# optional: --iterations 10
```

## maven/ (`arena-junit`)

```bash
bazel run //tools/benches/maven:bench_maven -- 6.1.0
# optional: 6.1.0 <iterations>
```

## dotnet/ (`ArenaDotnet.Xunit`)

```bash
bazel run //tools/benches/dotnet:bench_dotnet -- 6.1.0
# optional: 6.1.0 <iterations>
```

All three can also be run directly without Bazel (`./tools/benches/maven/run.sh 6.1.0`,
`./tools/benches/dotnet/run.sh 6.1.0`, `python3 tools/benches/pypi/bench_pypi.py
--version 6.1.0`) as long as `mvn` / `dotnet` / `python3` are on `PATH`.

## Running all three together

Each registry publishes pre-release identifiers differently (Maven snapshots use
`6.1.0-SNAPSHOT`, NuGet preview builds use `6.1.0-pr214-<sha>`, PyPI has its own
scheme), so `run_all.sh` takes a separate version per language:

```bash
bazel run //tools/benches:bench_all -- --python 6.1.0 --java 6.1.0 --dotnet 6.1.0
# optional: --iterations 10
# or directly: ./tools/benches/run_all.sh --python 6.1.0 --java 6.1.0 --dotnet 6.1.0
```

Runs all three in sequence and prints a table. If one language fails (e.g. that
version isn't published to its registry yet), the other two still print their
results, the failed one shows a `FAILED` row, and the script exits non-zero:

```
lang     version  iters   open_ms interact_min interact_med interact_p95  close_ms   e2e_ms
pypi     6.1.0       10   1626.15        18.40        22.09        29.11    639.46  2287.71
maven    6.1.0       10       ...
dotnet   6.1.0       10       ...
```

Compare `interact_ms` (median) and `e2e_ms` across a `6.1.0` run and a `6.2.0` run to
see the opt-level impact.
