# Arena

![Arena logo](./arena-logo.png)

[![CI](https://github.com/daveepope/arena/actions/workflows/build-test-publish-arena.yml/badge.svg?branch=master)](https://github.com/daveepope/arena/actions/workflows/build-test-publish-arena.yml)
[![codecov](https://codecov.io/gh/daveepope/arena/graph/badge.svg?branch=master)](https://codecov.io/gh/daveepope/arena)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![OpenSSF Scorecard](https://api.scorecard.dev/projects/github.com/daveepope/arena/badge)](https://scorecard.dev/viewer/?uri=github.com/daveepope/arena)
[![OSV Lockfile Scan](https://github.com/daveepope/arena/actions/workflows/osv-scanner.yml/badge.svg?branch=master)](https://github.com/daveepope/arena/actions/workflows/osv-scanner.yml)
[![Dependency Vulnerability Scan](https://github.com/daveepope/arena/actions/workflows/dependency-review.yml/badge.svg?branch=master)](https://github.com/daveepope/arena/actions/workflows/dependency-review.yml)
[![Supply Chain Protection](https://img.shields.io/github/actions/workflow/status/daveepope/arena/build-test-publish-arena.yml?branch=master&label=Supply%20Chain%20Protection%20(%3C3d))](https://github.com/daveepope/arena/actions/workflows/build-test-publish-arena.yml)
[![Best Effort Default Container CVE Search](https://github.com/daveepope/arena/actions/workflows/container-cves.yml/badge.svg?branch=master)](https://github.com/daveepope/arena/actions/workflows/container-cves.yml)

Client packages (all built from the same release, so their versions always match):

[![PyPI](https://img.shields.io/pypi/v/arena-pytest.svg?label=PyPI)](https://pypi.org/project/arena-pytest/)
[![Maven Central](https://img.shields.io/maven-central/v/io.github.stationdevx/arena-junit.svg?label=Maven%20Central)](https://central.sonatype.com/artifact/io.github.stationdevx/arena-junit)
[![NuGet](https://img.shields.io/nuget/v/ArenaDotnet.Xunit.svg?label=NuGet)](https://www.nuget.org/packages/ArenaDotnet.Xunit)

Arena is a cross-platform sandboxing framework. It manages the lifecycle of a set of dependencies (databases, brokers, HTTP services) and components (your applications) as a single sandbox you can open, interact with, and close, giving you repeatable, deterministic, multi-service environments with a fast feedback loop. Arena provides top-level clients for Python, Java, Go, and .NET (the Python client `arena-pytest`, Java client `arena-junit`, and .NET client `arena-xunit` all ship today; a Go client is planned). Component testing is one common use case, but Arena is equally at home as a local development sandbox, a scripted scenario driver, or anywhere else you need a reproducible multi-service environment.

## Overview

Arena models a sandbox using the concept of matches to manage the lifecycle of a set of dependencies and components (your applications).

The core framework is implemented in Rust. Clients call the core framework library through a C FFI layer which is completly hidden from application developers. The Python client (arena-pytest), Java client (arena-junit), and .NET client (arena-xunit) are available; a Go client is planned.

![Arena architecture](./arena.drawio.png)

## Performance

Arena is built for speed and efficiency. Within a match, all dependencies start concurrently using the simple concept of dependency trees where dependencies can declare children; the tree is respected so children start before parents and stop after them. This keeps setup and teardown time low even with many services. The same concept applies to component trees where one component starts before another where a dependency relationship exists.

Bazel build is used to build and runs tests in parallel and streams logs during execution. All runtimes are built and tested together, e.g. rust, python ett. For Cargo, use `cargo testv` or `cargo test -- --nocapture` to stream output. For pytest, use `-s` to disable capture.

## Prerequisites

- Bazel (via [Bazelisk](https://github.com/bazelbuild/bazelisk) is recommended)
- Docker

## Agent instructions (AI / editors)

- **`AGENTS.md` is the source of truth** for project agent rules (coding assistants, CI context, etc.).
- **`CLAUDE.md`** and **`.cursor/rules/arena-agent.mdc`** are **generated** from it: do not edit them by hand.
- After you change **`AGENTS.md`**, run **`bazel run //scripts:sync_agent_rules`**, then commit **`AGENTS.md`**, **`CLAUDE.md`**, and **`.cursor/rules/arena-agent.mdc`** together.

## Installation

Build the project:

```bash
bazel build //...
```

Run the full test suite:

```bash
bazel test //...
```

## Host development (Cargo and Python)

You can use Cargo and Python on your host machine instead of Bazel. You will need to install the prerequisites yourself: Rust 1.92+, Python 3.9+, and the dependencies in `arena-pytest/requirements.txt`. Build the FFI library with `cargo build -p arena-ffi --release`, then install the Python client with `pip install -e arena-pytest`.

## Usage

### Rust

```rust
use arena::{ClosedArena, Component, Dependency, Match, MatchTrait};
use arena_executable_component::executable_component::ExecutableComponent;
use arena_kafka::{KafkaDependency, KafkaFlavor};
use arena_postgres::PostgresDependency;

let postgres: Dependency = Box::new(
    PostgresDependency::builder("readings")
        .with_port(5432)
        .with_database_name("mydb")
        .build(),
);

let kafka: Dependency = Box::new(
    KafkaDependency::builder("readings")
        .with_flavor(KafkaFlavor::ApacheNative)
        .with_port(9092)
        .with_topic("events")
        .build(),
);

let web_app: Component = Box::new(
    ExecutableComponent::builder("my service")
        .with_executable_path("/path/to/your/binary")
        .build(),
);

let a_match = Match::new("my-test", vec![postgres, kafka], vec![web_app]);
let closed = ClosedArena::new("Arena".to_string(), vec![Box::new(a_match)]);
let open = closed.open().await;

// Interact with the open arena
// ...

open.close().await;
```

You can also use `with_source_path` / `with_build_tool` on the builder so Arena builds the binary before starting it (see `examples/`).

**Playbooks**: register a `Managed*Playbook` (or your own `Playbook` implementation for unmanaged behavior) on the `Match` via `register_playbook(playbook, exec_on_dependency_start)`, then open scoped ones yourself with `dependency.playbook().run().await` when a test needs them. See [Playbooks](#playbooks) below for the full picture.

### Python (arena-pytest)

Published on PyPI: [arena-pytest](https://pypi.org/project/arena-pytest/)

```python
from arena_pytest import (
    ClosedArena,
    MatchBuilder,
    ExecutableComponentBuilder,
    HttpReadinessCheck,
    KafkaDependencyBuilder,
    KafkaFlavor,
    PostgresDependencyBuilder,
)

postgres = (
    PostgresDependencyBuilder("db")
    .with_port(5432)
    .with_database_name("mydb")
    .build()
)

kafka = (
    KafkaDependencyBuilder("kafka")
    .with_flavor(KafkaFlavor.APACHE_NATIVE)
    .with_port(9092)
    .with_topic("events")
    .build()
)

component = (
    ExecutableComponentBuilder("my service")
    .with_executable_path("/path/to/your/binary")
    .with_readiness_check(HttpReadinessCheck(), "http://127.0.0.1:8080/health")
    .build()
)

a_match = (
    MatchBuilder("my-test")
    .add_dependency(postgres)
    .add_dependency(kafka)
    .add_component(component)
    .build()
)

closed = ClosedArena("Arena", [a_match])
open_arena = await closed.open()

# Interact with open_arena
# ...

await open_arena.close()
```

As in Rust, you can point at source plus `with_build_tool(...)` instead of a prebuilt path when you want Arena to compile the component first.

#### Playbooks

Register a `Managed*Playbook` (or your own class extending `Playbook`/`UnmanagedPlaybook`) on the `MatchBuilder` alongside your dependencies, then stack `@playbook(YourPlaybook)` decorators on a test to activate it just for that test:

```python
a_match = (
    MatchBuilder("my-test")
    .add_dependency(mssql)
    .register_playbook(
        ResetValidationDbPlaybook(mssql.identifier),
        exec_on_dependency_start=False,
    )
    .build()
)

@playbook(ResetValidationDbPlaybook)
def test_create_reading_with_validation_db_scoped_playbook(api_client):
    # the table is reset once this test finishes
    ...
```

See [Playbooks](#playbooks) below for the full picture, including how to write your own unmanaged playbook.

### Java (arena-junit)

Published on Maven Central: [io.github.stationdevx:arena-junit](https://central.sonatype.com/artifact/io.github.stationdevx/arena-junit)

`arena-junit` is a JUnit 5 extension. You point it at the jar your build already produces, so a test runs against the same artifact you ship rather than a separate in-process test context.

#### Setup

`arena-junit` publishes its native library as a separate Maven classifier per platform (`linux-x86_64`, `osx-aarch_64`, `osx-x86_64`) instead of bundling every platform into one jar. Add the [`os-maven-plugin`](https://github.com/trustin/os-maven-plugin) so the right classifier resolves automatically for your machine:

```xml
<build>
  <extensions>
    <extension>
      <groupId>kr.motd.maven</groupId>
      <artifactId>os-maven-plugin</artifactId>
      <version>1.7.1</version>
    </extension>
  </extensions>
</build>

<dependency>
  <groupId>io.github.stationdevx</groupId>
  <artifactId>arena-junit</artifactId>
  <version>${arena.version}</version>
  <classifier>${os.detected.classifier}</classifier>
</dependency>
```

Gradle (Kotlin DSL), using the [`osdetector`](https://github.com/google/osdetector-gradle-plugin) plugin:

```kotlin
plugins {
  id("com.google.osdetector") version "1.7.3"
}

dependencies {
  testImplementation("io.github.stationdevx:arena-junit:$arenaVersion:${osdetector.classifier}")
}
```

#### Annotation style

Put `@Arena` on the test class, annotate your dependencies and components as static fields, and Arena wires the sandbox for you before any test method runs.

```java
@Arena
final class ReadingsComponentTest {

  @ArenaDependency
  static final PostgresDependency POSTGRES =
      new PostgresDependencyBuilder("readings-db")
          .withPort(5432)
          .withDatabaseName("readings")
          .withDatabaseUsername("readings_user")
          .withDatabasePassword("readings_password")
          .build();

  @ArenaComponent
  static final ExecutableComponent WEB_APP =
      new ExecutableComponentBuilder("readings-app")
          .withExecutablePath("/path/to/readings-app.jar")
          .withEnvVar("POSTGRES_CONNECTION_STRING", "host=localhost port=5432 dbname=readings")
          .withReadinessCheck(HttpReadinessCheck.create(), "http://127.0.0.1:8080/health")
          .build();

  @Test
  void createReadingIsListed() throws Exception {
    // call the app over HTTP, same as any other component test
  }
}
```

Postgres and the app start at the same time rather than one after the other, so adding another dependency does not add its startup time on top of the rest.

Need a second copy of your service, or a second service alongside it, for a domain test or a scale test? Add another field:

```java
@ArenaComponent
static final ExecutableComponent WEB_APP_2 =
    new ExecutableComponentBuilder("readings-app-2")
        .withExecutablePath("/path/to/readings-app.jar")
        .withEnvVar("POSTGRES_CONNECTION_STRING", "host=localhost port=5432 dbname=readings")
        .withReadinessCheck(HttpReadinessCheck.create(), "http://127.0.0.1:8081/health")
        .build();
```

Both instances run in the same sandbox against the same Postgres, so you can test how your service behaves with two copies of itself running, or bring in another team's service and test the two together.

#### Building the arena yourself

If you would rather not use field scanning, build the same sandbox by hand with `MatchBuilder` and open it in a plain JUnit lifecycle method:

```java
final class ReadingsComponentTest {

  private static OpenArena openArena;

  @BeforeAll
  static void openArena() throws Exception {
    PostgresDependency postgres =
        new PostgresDependencyBuilder("readings-db")
            .withPort(5432)
            .withDatabaseName("readings")
            .build();

    ExecutableComponent webApp =
        new ExecutableComponentBuilder("readings-app")
            .withExecutablePath("/path/to/readings-app.jar")
            .withReadinessCheck(HttpReadinessCheck.create(), "http://127.0.0.1:8080/health")
            .build();

    Match match =
        new MatchBuilder("readings")
            .addDependency(postgres)
            .addComponent(webApp)
            .build();

    openArena = new ClosedArena("readings-arena", List.of(match)).open();
  }

  @AfterAll
  static void closeArena() {
    openArena.close();
  }

  @Test
  void createReadingIsListed() throws Exception {
    // call the app over HTTP, same as any other component test
  }
}
```

Use this when you want full control over when the sandbox opens and closes, for example sharing one arena across several test classes yourself instead of letting `@Arena` manage it.

#### Playbooks

Declare a `Managed*Playbook` (or your own class implementing `Playbook`/`UnmanagedPlaybook`) as an `@ArenaPlaybook` static field alongside your dependencies, then stack `@Playbook(YourPlaybook.class)` on a `@Test` method to activate it just for that test:

```java
@ArenaPlaybook(execOnDependencyStart = false)
static final ResetValidationDbPlaybook RESET_VALIDATION_DB =
    new ResetValidationDbPlaybook(MSSQL.identifier());

@Test
@Playbook(ResetValidationDbPlaybook.class)
void createReadingWithValidationDbScopedPlaybook() throws Exception {
  // the table is reset once this test finishes
}
```

See [Playbooks](#playbooks) below for the full picture, including how to write your own unmanaged playbook.

### .NET (arena-xunit)

Published on NuGet: [ArenaDotnet.Xunit](https://www.nuget.org/packages/ArenaDotnet.Xunit)

`arena-xunit` is an xUnit v2 extension targeting `netstandard2.0`, so it works from both .NET Framework and modern .NET test projects. You point it at the executable your build already produces, so a test runs against the same artifact you ship rather than a separate in-process test context.

#### Xunit annotations

Put your dependencies and components as `[ArenaDependency]` / `[ArenaComponent]` static fields on a class that extends `ArenaCollectionFixture`, then have your test class implement xUnit's `IClassFixture<T>` to receive the shared, already-open arena before any test method runs.

```csharp
public sealed class ReadingsFixture : ArenaCollectionFixture
{
    [ArenaDependency]
    private static readonly PostgresDependency Postgres =
        new PostgresDependencyBuilder("readings-db")
            .WithPort(5432)
            .WithDatabaseName("readings")
            .WithDatabaseUsername("readings_user")
            .WithDatabasePassword("readings_password")
            .Build();

    [ArenaComponent]
    private static readonly ExecutableComponent WebApp =
        new ExecutableComponentBuilder("readings-app")
            .WithExecutablePath("/path/to/readings-app.dll")
            .WithEnvVar("POSTGRES_CONNECTION_STRING", "host=localhost port=5432 dbname=readings")
            .WithReadinessCheck(HttpReadinessCheck.Create(), "http://127.0.0.1:8080/health")
            .Build();
}

public class ReadingsComponentTests : IClassFixture<ReadingsFixture>
{
    private readonly OpenArena _arena;

    public ReadingsComponentTests(ReadingsFixture fixture)
    {
        _arena = fixture.Arena;
    }

    [Fact]
    public async Task CreateReadingIsListed()
    {
        // call the app over HTTP, same as any other component test
    }
}
```

Postgres and the app start at the same time rather than one after the other, so adding another dependency does not add its startup time on top of the rest.

Need a second copy of your service, or a second service alongside it, for a domain test or a scale test? Add another field:

```csharp
[ArenaComponent]
private static readonly ExecutableComponent WebApp2 =
    new ExecutableComponentBuilder("readings-app-2")
        .WithExecutablePath("/path/to/readings-app.dll")
        .WithEnvVar("POSTGRES_CONNECTION_STRING", "host=localhost port=5432 dbname=readings")
        .WithReadinessCheck(HttpReadinessCheck.Create(), "http://127.0.0.1:8081/health")
        .Build();
```

Both instances run in the same sandbox against the same Postgres, so you can test how your service behaves with two copies of itself running, or bring in another team's service and test the two together.

#### Building the arena yourself

If you would rather not use field scanning, build the same sandbox by hand with `MatchBuilder` and open it yourself in an xUnit fixture:

```csharp
public sealed class ReadingsFixture : IAsyncLifetime
{
    public OpenArena Arena { get; private set; } = null!;

    public async Task InitializeAsync()
    {
        var postgres = new PostgresDependencyBuilder("readings-db")
            .WithPort(5432)
            .WithDatabaseName("readings")
            .Build();

        var webApp = new ExecutableComponentBuilder("readings-app")
            .WithExecutablePath("/path/to/readings-app.dll")
            .WithReadinessCheck(HttpReadinessCheck.Create(), "http://127.0.0.1:8080/health")
            .Build();

        var match = new MatchBuilder("readings")
            .AddDependency(postgres)
            .AddComponent(webApp)
            .Build();

        Arena = await new ClosedArena("readings-arena", match).OpenAsync();
    }

    public Task DisposeAsync()
    {
        Arena.Dispose();
        return Task.CompletedTask;
    }
}

public class ReadingsComponentTests : IClassFixture<ReadingsFixture>
{
    private readonly OpenArena _arena;

    public ReadingsComponentTests(ReadingsFixture fixture)
    {
        _arena = fixture.Arena;
    }

    [Fact]
    public async Task CreateReadingIsListed()
    {
        // call the app over HTTP, same as any other component test
    }
}
```

Use this when you want full control over when the sandbox opens and closes, for example sharing one arena across several test classes yourself instead of letting `ArenaCollectionFixture` manage it via attributes.

#### Playbooks

Declare a `Managed*Playbook` (or your own class extending `UnmanagedPlaybook`) as an `[ArenaPlaybook]` static field alongside your dependencies, then stack `[Playbook(typeof(YourPlaybook))]` on a test method to activate it just for that test:

```csharp
[ArenaPlaybook(ExecOnDependencyStart = false)]
private static readonly ResetValidationDbPlaybook ResetValidationDb =
    new ResetValidationDbPlaybook(Mssql.Identifier);

[Fact]
[Playbook(typeof(ResetValidationDbPlaybook))]
public async Task CreateReadingWithValidationDbScopedPlaybook()
{
    // the table is reset once this test finishes
}
```

See [Playbooks](#playbooks) below for the full picture, including how to write your own unmanaged playbook.

## Playbooks

A **playbook** is a named, scoped behavior attached to a dependency in your sandbox. It describes how that dependency should act for the lifetime of the playbook, for example baseline HTTP responses for a downstream service, resetting MSSQL tables when a scenario begins and ends, purging localstack resources between scenarios, or seeding a row before a scenario runs. Playbooks are part of Arena’s lifecycle model: you open them when a scenario needs them and close them when that scenario is done, so the sandbox returns to a known baseline.

When a playbook is **active**, Arena runs its setup on open and its teardown on close (explicit close, scope exit, or arena shutdown). Some dependency types can also verify that expected interaction occurred during the playbook’s lifetime when the playbook declares those rules.

There are two kinds of playbook, and every dependency-backed playbook is one or the other:

- **[Managed playbooks](#managed-playbooks)**: you describe the behavior as a manifest (mappings, table resets, purge rules); Arena applies it on open and cleans up after itself on close. No hand-rolled teardown.
- **[Unmanaged playbooks](#unmanaged-playbooks)**: you write the behavior yourself as arbitrary code (seed a row, call another service, anything a manifest can't express). Arena just calls your code and disposes the handle you return; any teardown your scenario needs is on you.

Both kinds register on a match the same way and activate through the same scoped-activation mechanism (see [Scoped activation](#scoped-activation)); the difference is only who owns the setup/teardown logic, and, by default, whether the playbook runs before or after the test body.

### Managed playbooks

A **managed playbook** is a playbook whose behavior is declared up front as a manifest (mappings, table resets, purge rules, and similar). You **register** managed playbooks on a **match** when you build the sandbox. Arena applies the manifest when the playbook opens and **cleans up after itself when it closes** (mappings removed, tables reset, queues purged, and so on), so you do not hand-roll teardown. That automatic setup and teardown is what **managed** means. By default a managed playbook activates **after** the test body (teardown for whatever ran), so the next scenario starts clean.

Define sandbox-specific playbooks by **extending** the managed base for the dependency type (`ManagedHttpPlaybook`, `ManagedMssqlPlaybook`, `ManagedLocalstackPlaybook`, `ManagedPostgresPlaybook`, and similar in Python, Java, and .NET). In Rust, build the same manifests with the `Managed*Playbook` types from the dependency crates. Register the instance on the match; Arena executes it through the core runtime.

Register with **`exec_on_dependency_start`** (Python/Java), the `ExecOnDependencyStart` property on `[ArenaPlaybook]` (.NET), or the second argument to **`register_playbook`** (Rust):

- **`true`**: run when the dependency starts and stay active for the sandbox session (typical for default dependency behavior, such as a baseline HTTP stub for the whole run).
- **`false`**: register only; open when you need a shorter-lived scenario.

**Rust**: pass `Box<dyn Playbook>` from `ManagedHttpPlaybook`, `ManagedMssqlPlaybook`, or sibling types to `Match::register_playbook`.

**Python**: subclass a `Managed*Playbook` type and pass an instance to `MatchBuilder.register_playbook`.

**Java**: subclass a `Managed*Playbook` type and pass an instance to `MatchBuilder.registerPlaybook`.

**.NET**: subclass a `Managed*Playbook` type and pass an instance to `MatchBuilder.RegisterPlaybook`, or declare it as an `[ArenaPlaybook]` static field alongside your dependencies.

### Unmanaged playbooks

An **unmanaged playbook** is a playbook whose behavior is entirely your own code; there is no manifest, Arena does not know or manage what your `run` actually does. Use it for anything a manifest can't express: seeding a row before a test, calling another service, arbitrary setup. Implement `identifier()` and `run(arena)` yourself and return an active handle; Arena disposes that handle when the playbook closes, but whatever your code did (e.g. rows it inserted) is your own responsibility to clean up if the next scenario needs a clean slate. Pair it with a managed playbook's automatic teardown (see the stacked examples below) if you need that.

By default an unmanaged playbook activates **before** the test body, so the state it sets up is already there when the test starts, the opposite default from managed playbooks.

**Rust**: there is no separate `Unmanaged` marker; implement the `Playbook` trait directly (any hand-written implementation is, by definition, unmanaged) and pass `Box<dyn Playbook>` to `Match::register_playbook`.

**Python**: subclass `UnmanagedPlaybook`:

```python
class SeedValidationReadingPlaybook(UnmanagedPlaybook):
    def __init__(self, connection_string: str):
        self._connection_string = connection_string

    def identifier(self) -> str:
        return "seed-validation-reading"

    def run(self, arena) -> ActivePlaybook:
        # arbitrary setup code, e.g. seed a row via SQL
        ...
        return ActivePlaybook(None, 0)
```

**Java**: implement `Playbook` and `UnmanagedPlaybook`:

```java
public final class SeedValidationReadingPlaybook implements Playbook, UnmanagedPlaybook {
  @Override
  public String identifier() {
    return "seed-validation-reading";
  }

  @Override
  public ActivePlaybook run(OpenArena arena) {
    // arbitrary setup code, e.g. seed a row via JDBC
    return new NoopActivePlaybook();
  }
}
```

**.NET**: extend `UnmanagedPlaybook`:

```csharp
private class SeedValidationReadingPlaybook : UnmanagedPlaybook
{
    public override string Identifier => "seed-validation-reading";

    public override ActivePlaybook Run(OpenArena arena)
    {
        // arbitrary setup code, e.g. seed a row via SQL
        return new ActiveHttpPlaybook(IntPtr.Zero);
    }
}
```

Register and activate it exactly like a managed playbook: see [Scoped activation](#scoped-activation).

### Scoped activation

For playbooks registered with **`exec_on_dependency_start=false`**, open them only for the period that scenario should apply:

- **Rust**: obtain the dependency from the open arena, call `.playbook().run().await`, and hold the active playbook until scope ends.
- **Python**: stack `@playbook(YourPlaybook)` decorators on the callable that should run under that behavior (one playbook class per line).
- **Java**: stack `@Playbook(YourPlaybook.class)` annotations the same way (one class per line).
- **.NET**: stack `[Playbook(typeof(YourPlaybook))]` attributes on the test method or class the same way, with `[assembly: PlaybookExecutionAttribute]` declared once per test assembly.

Managed and unmanaged playbooks can be stacked together on the same test: Arena runs the unmanaged ones first (before the test body) and the managed ones last (after), regardless of the order you list them in. Session-default and scoped playbooks can coexist on one match. Scoped playbooks tear down when they close so the next scenario starts from a clean sandbox state.

## License

MIT. See [LICENSE](LICENSE).

This project may not be used to train AI models. See [AI.md](AI.md).

## Creator and Author
David Pope

## Contributing

Contributions are welcome and encouraged. Open an issue or pull request. Ensure tests pass before submitting. While its okay to use AI to assist in development, please do not submit PRs completely created by AI as these tend to contain incohernet changes (aka hallucinations/ slop). Thank you kindly!
