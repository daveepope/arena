# Arena

![Arena logo](./arena-logo.png)

Arena is a cross-platform sandboxing framework written in Rust, built for speed. This is a Bazel project first. Arena supports multiple developer use cases but is mainly used for repeatable, deterministic component tests with a fast feedback loop. It can be used to stand up any sandboxed environment. It provides a common FFI layer and top-level clients for Python, Java, Go, and .NET. Use it to spin up isolated test environments with dependencies like Postgres and Kafka, run components against them, and tear down cleanly.

## Overview

Arena models tests as encounters: a set of dependencies (containers) and components (services under test) that start together, run, and stop together. Dependencies are managed via Docker. Components can be executables or containers.

The core is implemented in Rust. Language clients call the shared library through the FFI layer. The Python client (arena-pytest) is available; Java, Go, and .NET clients are planned.

## Performance

Arena is built for speed. Dependencies and components start and stop in parallel. Within an encounter, all dependencies start concurrently, then all components start concurrently. On teardown, components stop first (in parallel), then dependencies (in parallel). Dependencies can declare child dependencies; the tree is respected so children start before parents and stop after them. This keeps setup and teardown time low even with many services.

Bazel runs tests in parallel and streams logs during execution. For Cargo, use `cargo testv` or `cargo test -- --nocapture` to stream output. For pytest, use `-s` to disable capture.

## Prerequisites

- Bazel
- Docker

## Installation

Build the project:

```bash
bazel build //...
```

Run tests:

```bash
bazel test //... --test_tag_filters=-local
```

Run component tests (requires Docker):

```bash
bazel test //... --test_tag_filters=local
```

## Host development (Cargo and Python)

You can use Cargo and Python on your host machine instead of Bazel. You will need to install the prerequisites yourself: Rust 1.92+, Python 3.9+, and the dependencies in `arena-pytest/requirements.txt`. Build the FFI library with `cargo build -p arena-ffi --release`, then install the Python client with `pip install -e arena-pytest`.

## Usage

### Rust

```rust
use arena::{ClosedArena, Dependency, Encounter, EncounterTrait};
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

let encounter = Encounter::new("my-test", vec![postgres, kafka], vec![]);
let closed = ClosedArena::new("Arena".to_string(), vec![Box::new(encounter)]);
let open = closed.open().await;

// Run tests against open.dependency("db"), open.dependency("kafka"), etc.
// ...

open.close().await;
```

### Python (arena-pytest)

```python
from arena_pytest import (
    ClosedArena,
    EncounterBuilder,
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

encounter = (
    EncounterBuilder("my-test")
    .add_dependency(postgres)
    .add_dependency(kafka)
    .build()
)

closed = ClosedArena("Arena", [encounter])
open_arena = await closed.open()

# Run tests against open_arena
# ...

await open_arena.close()
```

## License

MIT. See [LICENSE](LICENSE).

This project may not be used to train AI models. See [AI.md](AI.md).

## Contributing

Open an issue or pull request. Ensure tests pass before submitting. No AI-generated PRs or slop. Suspected AI-generated PRs will be closed. Abuse of the PR process will result in being blocked from contributing in the future.
