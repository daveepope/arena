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

## Playbooks

A **playbook** is a named, scoped behavior attached to a dependency in your sandbox. It describes how that dependency should act for the lifetime of the playbook — for example, baseline HTTP responses for a downstream service, resetting MSSQL tables when a scenario begins and ends, or purging localstack resources between scenarios. Playbooks are part of Arena’s lifecycle model: you open them when a scenario needs them and close them when that scenario is done, so the sandbox returns to a known baseline.

When a playbook is **active**, Arena applies its setup on open and its teardown on close (explicit close, scope exit, or arena shutdown). Some dependency types can also verify that expected interaction occurred during the playbook’s lifetime when the playbook declares those rules.

### Managed playbooks

A **managed playbook** is a playbook whose behavior is declared up front as a manifest (mappings, table resets, purge rules, and similar). You **register** managed playbooks on a **match** when you build the sandbox. Arena applies the manifest when the playbook opens and **cleans up after itself when it closes** — mappings removed, tables reset, queues purged, and so on — so you do not hand-roll teardown. That automatic setup and teardown is what **managed** means.

Define sandbox-specific playbooks by **extending** the managed base for the dependency type (`ManagedHttpPlaybook`, `ManagedMssqlPlaybook`, `ManagedLocalstackPlaybook`, and similar in Python and Java). In Rust, build the same manifests with the `Managed*Playbook` types from the dependency crates. Register the instance on the match; Arena executes it through the core runtime.

Register with **`exec_on_dependency_start`** (Python/Java) or the second argument to **`register_playbook`** (Rust):

- **`true`** — run when the dependency starts and stay active for the sandbox session (typical for default dependency behavior, such as a baseline HTTP stub for the whole run).
- **`false`** — register only; open when you need a shorter-lived scenario.

**Rust** — pass `Box<dyn Playbook>` from `ManagedHttpPlaybook`, `ManagedMssqlPlaybook`, or sibling types to `Match::register_playbook`.

**Python** — subclass a `Managed*Playbook` type and pass an instance to `MatchBuilder.register_playbook`.

**Java** — subclass a `Managed*Playbook` type and pass an instance to `MatchBuilder.registerPlaybook`.

### Scoped activation

For playbooks registered with **`exec_on_dependency_start=false`**, open them only for the period that scenario should apply:

- **Rust** — obtain the dependency from the open arena, call `.playbook().run().await`, and hold the active playbook until scope ends.
- **Python** — stack `@playbook(YourPlaybook)` decorators on the callable that should run under that behavior (one playbook class per line).
- **Java** — stack `@Playbook(YourPlaybook.class)` annotations the same way (one class per line).

Session-default and scoped playbooks can coexist on one match. Scoped playbooks tear down when they close so the next scenario starts from a clean sandbox state.

## License

MIT. See [LICENSE](LICENSE).

This project may not be used to train AI models. See [AI.md](AI.md).

## Creator and Author
David Pope

## Contributing

Contributions are welcome and encouraged. Open an issue or pull request. Ensure tests pass before submitting. While its okay to use AI to assist in development, please do not submit PRs completely created by AI as these tend to contain incohernet changes (aka hallucinations/ slop). Thank you kindly!
