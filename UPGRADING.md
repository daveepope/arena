# Upgrading from 6.2.1 to 7.0.0

7.0.0 replaces panics with a lifecycle fault contract. Every failure lands in the arena state, clients raise a typed lifecycle error carrying that state, and the arena guarantees teardown of everything it started. Most test suites need no code changes; work through the sections below that apply to you.

## All clients (Python, Java, .NET)

**Upgrade the package only.** The native library ships inside each package and must match the client version. If you pin a native yourself (for example via `ARENA_FFI_LIB`), replace it: the C ABI changed (`arena_open` and `arena_close` signatures, plus new state and observer functions), and a 6.x native will not work.

**Failed opens raise `ArenaLifecycleError`.** It subclasses the existing binding error and carries the parsed arena state (`error.state` / `error.state()` / `error.State`), which includes every dependency, component, and recorded fault. Update any code that caught the old error types or matched old message text. Panic text such as `panicked at ...` no longer appears anywhere; you get the rendered arena state instead.

**Log output changed shape.** Lines now log under object namespaces: `arena.<id>`, `arena.<id>.dependency.<id>`, `arena.<id>.component.<id>`. The `[arena::module]` message prefix is gone, fields are ` | ` separated, fields repeating the id already in the logger name are dropped, and durations are rounded. Update logger configuration keyed on old names (the old `arena.rust.dispatcher` logger is now `arena`) and anything that greps log lines. Lifecycle transitions and a `closing summary | state=<state> | faults=<n>` line are logged on every run.

**Resets can fail loudly.** A `soft_reset` / `hard_reset` that was failing silently in 6.2.1 now raises with the real fault.

**Containers expire.** Container dependencies stamp a five minute expiry by default and clean up their own module's expired containers on start. An arena held open longer than that needs `with_expiry(...)` or `without_expiry()`. Container names also changed for identifiers whose last segment is six characters (for example `oracle`, `broker`, `server`); update anything pinning those exact names.

## Python (arena-pytest)

- A failed open propagates `ArenaLifecycleError` from the fixture instead of calling `pytest.fail`, so `pytest.raises` and exception assertions see the real type.
- A `SIGTERM` during a run ends the session so fixture teardown closes the arena.

## Java (arena-junit)

- A failed open throws `ArenaLifecycleError` (a `RuntimeException` via `ArenaBindingError`) instead of `IllegalStateException`; only configuration mistakes (bad annotations, missing fields) still throw `IllegalStateException`.
- The JVM shutdown hook closes any arena a suite left open and logs failures instead of throwing.

## .NET (arena-xunit)

- `OpenAsync` throws `ArenaLifecycleError`; catching `ArenaBindingError` still works.
- `OpenArena.Dispose` now closes the arena itself and throws close faults it previously swallowed in the handle finalizer. Multiple failures aggregate under the message `arena close completed with failures`.
- Open arenas are closed on process exit and Ctrl-C.

## Rust

- `RunnableDependency` and `RunnableComponent` lifecycle methods return `Result<(), Fault>` and gain `state`, `faults`, `force_stop`, and `release`; `RunnableComponent` also gains `identifier`, `children`, and `children_mut`. Custom implementations must be updated.
- `ClosedArena::open` returns `Result<OpenArena, ArenaState>`, `OpenArena::close` returns `Result<ClosedArena, ArenaState>`; handle the `Err` instead of catching panics.
- `Playbook::run` returns `Result<Box<dyn ActivePlaybook>, Fault>`; `OpenArena::run_playbook` returns `Option<Result<..>>`.
- `OauthDependencyBuilder`, `HttpDependencyBuilder`, `OracleDependencyBuilder`, and `ExecutableComponentBuilder`: `build()` returns `Result<_, Fault>`. Append `?` or `.expect(...)` at call sites; former panic tests should assert the `Err`.
- `ManagedHttpPlaybook::new` takes a build closure returning `Result<Playbook, Fault>`; wrap infallible closures in `Ok`.
- `MatchTrait::start`/`stop` take `&LifecycleContext` and return `Result<(), Vec<Fault>>`; `force_stop_all` returns the faults from the forced sweep.
- The public `*Impl` traits (`HttpImpl`, `KafkaImpl`, `MssqlImpl`, and the rest) return `Result<(), String>` from `start`/`stop` and gain `force_stop` and `release`.
- `ClosedArena` is identified by `id` rather than `name`.
