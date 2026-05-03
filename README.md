# Arena

![Arena logo](./arena-logo.png)

[![Build, test, publish arena](https://github.com/daveepope/arena/actions/workflows/build-test-publish-arena.yml/badge.svg)](https://github.com/daveepope/arena/actions/workflows/build-test-publish-arena.yml)

Arena is a cross-platform sandboxing framework. It manages the lifecycle of a set of dependencies (databases, brokers, HTTP services) and components (your applications) as a single sandbox you can open, interact with, and close — giving you repeatable, deterministic, multi-service environments with a fast feedback loop. Arena provides top-level clients for Python, Java, Go, and .NET (the Python client `arena-pytest` ships today; Java, Go, and .NET clients are planned). Component testing is one common use case, but Arena is equally at home as a local development sandbox, a scripted scenario driver, or anywhere else you need a reproducible multi-service environment.

## Overview

Arena models a sandbox using the concept of matches to manage the lifecycle of a set of dependencies and components (your applications).

The core framework is implemented in Rust. Clients call the core framework library through a C FFI layer which is completly hidden from application developers. The Python client (arena-pytest) is available; Java, Go, and .NET clients are planned.

## Performance

Arena is built for speed and efficiency. Within a match, all dependencies start concurrently using the simple concept of dependency trees where dependencies can declare children; the tree is respected so children start before parents and stop after them. This keeps setup and teardown time low even with many services. The same concept applies to component trees where one component starts before another where a dependency relationship exists.

Bazel build is used to build and runs tests in parallel and streams logs during execution. All runtimes are built and tested together, e.g. rust, python ett. For Cargo, use `cargo testv` or `cargo test -- --nocapture` to stream output. For pytest, use `-s` to disable capture.

## Prerequisites

- Bazel (via [Bazelisk](https://github.com/bazelbuild/bazelisk) is recommended)
- Docker

> Note: hosted GitHub macOS runners don't provide a Docker daemon, so CI's macOS leg skips the Docker-backed tests by passing `--test_tag_filters=-local`. Linux CI and local development (including macOS with Docker Desktop) run the full suite.

## Agent instructions (AI / editors)

- **`AGENTS.md` is the source of truth** for project agent rules (coding assistants, CI context, etc.).
- **`CLAUDE.md`** and **`.cursor/rules/arena-agent.mdc`** are **generated** from it — do not edit them by hand.
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

#### HTTP playbooks (Rust)

`arena-http` and `arena-mssql` both ship a centralized **playbook** API with a lifetime scope, so per-scenario setup and teardown happen implicitly. Calling `.run().await` returns an `ActivePlaybook`: setup runs eagerly (HTTP mappings registered, MSSQL tables reset), and teardown runs automatically when the value is dropped (HTTP mappings removed and `expect_called(...)` expectations verified; MSSQL tables reset again so the next scenario starts clean). Bind the active playbook to a scoped variable and let scope exit do the cleanup.

When a scenario needs a dependency to behave differently (e.g. an outage, a bad response), grab the `HttpDependency` from the open arena and run a **playbook**. Mappings registered by the playbook are scoped to its lifetime and automatically removed on drop; expectations declared via `.expect_called(...)` are verified on drop and panic if unmet.

```rust
use arena_http::{HttpDependency, server_error};

let calibration = open
    .dependency("calibration")
    .and_then(|d| d.as_any().downcast_ref::<HttpDependency>())
    .expect("calibration available");

{
    let _outage = calibration
        .playbook()
        .post("/api/v1/validate")
            .with_priority(1)
            .will_return(server_error())
            .expect_called(1)
        .run()
        .await;

    // requests to /api/v1/validate now get 500 — exercise the failure path.
}
// _outage dropped here: mapping removed, expectation verified.
```

### Python (arena-pytest)

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

#### HTTP playbooks (Python)

The Python client mirrors the Rust playbook API. Use `HttpPlaybookBuilder` with an open arena and a `with` block for scoped setup and teardown; unmet `expect_called` expectations raise `AssertionError` on exit. The example below uses an `arena` pytest fixture, which is one convenient way to get an open arena, but any open arena works.

```python
from arena_pytest import HttpPlaybookBuilder

def test_calibration_outage_returns_500(arena):
    outage = (
        HttpPlaybookBuilder("calibration")
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
        r = requests.post("http://127.0.0.1:3001/readings", json={...})
        assert r.status_code == 500
    # outage context exits: mapping removed, expectation verified.
```

## License

MIT. See [LICENSE](LICENSE).

This project may not be used to train AI models. See [AI.md](AI.md).

## Creator and Author
David Pope

## Contributing

Contributions are welcome and encouraged. Open an issue or pull request. Ensure tests pass before submitting. While its okay to use AI to assist in development, please do not submit PRs completely created by AI as these tend to contain incohernet changes (aka hallucinations/ slop). Thank you kindly!
