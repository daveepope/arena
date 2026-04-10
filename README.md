# Arena

![Arena logo](./arena-logo.png)

[![Build, test, publish arena](https://github.com/daveepope/arena/actions/workflows/build-test-publish-arena.yml/badge.svg)](https://github.com/daveepope/arena/actions/workflows/build-test-publish-arena.yml)

Arena is a cross-platform sandboxing framework. Arena supports multiple developer focused use cases. While the core Arena framework is not a testing framework it was designed to streamline the creation of repeatable, deterministic component tests with a fast feedback loop. It can be used to stand up sandboxed environments outside of testing scenarios also. It provides top-level clients for Python, Java, Go, and .NET. These top level clients include plugins and extenstions for popular unit testing frameworks.

## Overview

Arena models a sandbox ussing the concept of encounters to manage the lifecycle of a set of dependencies and components (applications under test).

The core framework is implemented in Rust. Clients call the core framework library through a C FFI layer which is completly hidden from application developers. The Python client (arena-pytest) is available; Java, Go, and .NET clients are planned.

## Performance

Arena is built for speed and efficiency. Within an encounter, all dependencies start concurrently using the simple concept of dependency trees where dependencies can declare children; the tree is respected so children start before parents and stop after them. This keeps setup and teardown time low even with many services. The same concept applies to component trees where one component starts before another where a dependency relationship exists.

Bazel build is used to build and runs tests in parallel and streams logs during execution. All runtimes are built and tested together, e.g. rust, python ett. For Cargo, use `cargo testv` or `cargo test -- --nocapture` to stream output. For pytest, use `-s` to disable capture.

## Prerequisites

- Bazel (via [Bazelisk](https://github.com/bazelbuild/bazelisk) is recommended)
- Docker (only for **component** tests: targets tagged `local`)

<<<<<<< HEAD
**Platforms:** CI builds on **Linux and macOS**. **All** tests (including Docker-backed targets tagged `local`) run on **Linux** in CI. Hosted **macOS** runners have no Docker, so CI there runs everything **except** `local` tests. Locally on macOS, use Docker Desktop if you want to run the full suite.
=======
**Platforms:** The tree is built and unit-tested on **Linux and macOS** (see CI). Component tests need a working Docker daemon; on **macOS** use Docker Desktop (or another engine) locally. GitHub Actions runs component tests on **Linux only** because hosted macOS runners do not provide Docker.
>>>>>>> af03ce3957f7b0e258f2e89b74a0e87be8727009

## Installation

Build the project:

```bash
bazel build //...
```

Run **all** tests (requires Docker for targets tagged `local`, e.g. arena-pytest component tests):

```bash
bazel test //...
```

Run tests **without** Docker-heavy targets (faster when you have no daemon):

```bash
bazel test //... --test_tag_filters=-local
```

## Host development (Cargo and Python)

You can use Cargo and Python on your host machine instead of Bazel. You will need to install the prerequisites yourself: Rust 1.92+, Python 3.9+, and the dependencies in `arena-pytest/requirements.txt`. Build the FFI library with `cargo build -p arena-ffi --release`, then install the Python client with `pip install -e arena-pytest`.

## Usage

### Rust

```rust
use arena::{ClosedArena, Component, Dependency, Encounter, EncounterTrait};
use arena_executable_component::executable_component::ExecutableComponent;
use arena_kafka::{KafkaDependency, KafkaFlavor};
use arena_postgres::PostgresDependency;

let postgres: Dependency = Box::new(
    PostgresDependency::builder("db")
        .with_port(5432)
        .with_database_name("mydb")
        .build(),
);

let kafka: Dependency = Box::new(
    KafkaDependency::builder("kafka")
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

let encounter = Encounter::new("my-test", vec![postgres, kafka], vec![web_app]);
let closed = ClosedArena::new("Arena".to_string(), vec![Box::new(encounter)]);
let open = closed.open().await;

<<<<<<< HEAD
// Run tests against open arena
=======
// Run tests against open areana
>>>>>>> af03ce3957f7b0e258f2e89b74a0e87be8727009
// ...

open.close().await;
```

You can also use `with_source_path` / `with_build_tool` on the builder so Arena builds the binary before starting it (see `examples/`).

### Python (arena-pytest)

```python
from arena_pytest import (
    ClosedArena,
    EncounterBuilder,
    ExecutableComponentBuilder,
<<<<<<< HEAD
    HttpReadinessCheck,
=======
>>>>>>> af03ce3957f7b0e258f2e89b74a0e87be8727009
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

<<<<<<< HEAD
component = (
    ExecutableComponentBuilder("my service")
    .with_executable_path("/path/to/your/binary")
    .with_readiness_check(HttpReadinessCheck(), "http://127.0.0.1:8080/health")
=======
web_app = (
    ExecutableComponentBuilder("my service")
    .with_executable_path("/path/to/your/binary")
    .with_readiness_check_url("http://127.0.0.1:8080/health")
>>>>>>> af03ce3957f7b0e258f2e89b74a0e87be8727009
    .build()
)

encounter = (
    EncounterBuilder("my-test")
    .add_dependency(postgres)
    .add_dependency(kafka)
<<<<<<< HEAD
    .add_component(component)
=======
    .add_component(web_app)
>>>>>>> af03ce3957f7b0e258f2e89b74a0e87be8727009
    .build()
)

closed = ClosedArena("Arena", [encounter])
open_arena = await closed.open()

# Run tests against open_arena
# ...

await open_arena.close()
```

As in Rust, you can point at source plus `with_build_tool(...)` instead of a prebuilt path when you want Arena to compile the component first.

## License

MIT. See [LICENSE](LICENSE).

This project may not be used to train AI models. See [AI.md](AI.md).

## Creator and Author
David Pope

## Contributing

Contributions are welcome and encouraged. Open an issue or pull request. Ensure tests pass before submitting. While its okay to use AI to assist in development, please do not submit PRs completely created by AI as these tend to contain incohernet changes (aka hallucinations/ slop). Thank you kindly!
