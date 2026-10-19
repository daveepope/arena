# Changelog

All notable changes to Arena will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [7.1.0]

### Added

- `arena-profile`: the python sampler names frames with `co_qualname` where the interpreter has it and `co_name` otherwise, so it profiles any CPython from 3.7 up, refuses to arm below that or in a directory that is not private to this user, samples in the signal handler but flushes from its own serialised thread so nothing it does escapes into the application, skips its own flush thread, abandons sampling in a `fork()`ed child rather than writing over its parent's profile, reports no profile when it sampled nothing, disarms before the final flush, restores a cpu timer another library had already armed, and denies the profile outright when it cannot record why it refused
- `arena-profile`: unit coverage that the generated sampler carries no unfilled placeholder, interpolates only numbers, never names a host path, and that a refusal status cannot be forged by appending to it; plus component coverage that losing `SIGPROF` mid-run faults instead of reporting a profile
- `arena-profile`: `PythonSiteSampler`, a cpu profiler backend that samples inside the component's own interpreter through a generated `sitecustomize.py` on `PYTHONPATH`, selected for `BuildTool::Python` on macOS where py-spy needs root; it arms `SIGPROF` with `ITIMER_PROF`, collapses every thread's stack on each tick, flushes periodically so a SIGTERM loses at most one interval, caps distinct stacks and bytes against the existing cpu profile limit, and refuses rather than stealing a `SIGPROF` or timer another library already holds
- `arena-profile`: the sampler payload is compiled into the crate and written out at run time, so it needs no vendored tool, no runfiles and no PATH lookup, and ships identically in the pip wheel, maven jar and nuget package
- `arena-profile`: `ScratchDir` gives every profiled run a private 0700 directory with a random name and exclusively created 0600 files, refusing unless the directory is this process's at exactly 0700 and the temp root is either ours or sticky
- `arena-dashboard`: the server sends a `Content-Security-Policy` confining the page to its own origin, with `frame-ancestors`, `base-uri`, `form-action` and `object-src` all denied
- CI: profiler targets carry a `cpu_profile_test` tag that both macOS jobs run explicitly, since those jobs filter out `component_test` and would otherwise never exercise the profilers on the one platform that needs them; the Python and Java profiler targets are `no-windows` and the .NET one runs everywhere
- CI: Chrome for Testing is vendored for `mac-x64` and `mac-arm64` at the existing pin, so the dashboard browser test runs on macOS as well as linux x86_64
- `arena-dashboard`: browser coverage for opening a cpu profile report from a component's detail dialog, not only from its graph node
- `arena-profile`: a test proving a hostile frame name (`</text><script>` and an attribute break) is escaped before it reaches the rendered flamegraph
- `arena-profile`: async-profiler's macOS build is vendored alongside the linux ones, so Java components profile on macOS, resolving `libasyncProfiler.dylib` from runfiles with the same PATH fallback
- `arena-dashboard`: the server refuses any request whose `Host` or `Origin` is not loopback, and every mutating `POST` must carry the `x-arena-dashboard` caller header, so a page on another origin can no longer stop the server or wipe its state
- `arena-dashboard`: the arena count and the cpu profile count are capped, and a breach is refused with the limit named rather than evicting the oldest
- `arena-dashboard`: holding `Ctrl` (or `Cmd`) while scrolling zooms the canvas, with the page's own zoom cancelled over the canvas only, so a plain scroll still scrolls the page
- `examples`: the FastAPI suite carries py-spy and the .NET suites carry `dotnet-trace` as platform-selected test data, and the .NET example declares `WithBuildTool(BuildTool.Dotnet)` so its profiler backend resolves
- `arena-dashboard`: `GET /profiles` returns the profiler state the server holds and the page fetches it on load, so reloading no longer loses every profile report until the next event
- `arena-junit`, `arena-xunit`: `@ArenaVisualize(cpuProfile = {"component-id"})` and `[ArenaVisualize(CpuProfile = new[] { "component-id" })]` request a cpu profiler for the dashboard, matching what `with_cpu_profiler` already gave `arena-pytest`; the example suites visualize without it, since cpu profiling is a local workflow
- `arena-profile`, `arena-dashboard`: a rendered profile carries a second hotspot ranking crediting each runtime-leaf sample to its nearest application caller rather than dropping it, so an application function is credited the runtime time spent beneath it and reads higher there than in the leaf-only ranking, both over the same whole-profile denominator; runtime frames are matched against how .NET, the JVM, CPython and native code actually spell them, and the panel's `app only` toggle switches the table between the two rankings while leaving the flamegraph alone, with an info button naming the frames each language hides
- `arena`, `arena-dashboard`: each playbook counts how many times it has been run, kept on its lifecycle beside its state and faults under one lock and carried on every event, and the dashboard row names the count; every activation counts, including one that starts while another is still open and one whose body faults, and the count resets when the arena attaches its playbooks so reopening an arena starts from zero
- `arena`: playbooks report their own `PlaybookState` (`registered`, `running`, `stopped`, `faulted`) validated by a `PlaybookRule`, instead of borrowing the dependency and component `RunnableState`; a playbook has no readiness gate and no stopping step, so those states are gone from its channel, and the dashboard row reads `running`
- `arena`, `arena-dashboard`: the playbook channel drops the stale `Scoped` prefix now that startup playbooks report on it too (`PlaybookEvent`, `PlaybookLifecycle`, `PlaybookForwarder`, `observe_playbooks`, `PlaybookPanel`), and the channel is named `playbook`
- `arena`: `ManagedPlaybook` and `UnmanagedPlaybook` base types own a playbook's lifecycle and its start reporting, mirroring the client libraries' split; the five Arena-provided playbooks hold the managed one, leaving the FFI and every client untouched
- `arena`, `arena-dashboard`: playbook events carry `playbook_type`, stamped `managed` or `unmanaged` by the base class that built the playbook, and each dashboard row names it; the field stays inside the playbook event channel, so the FFI and the client libraries have no counterpart and a client-implemented playbook never reaches the Rust side to be labelled
- `arena`: playbook events name their kind, `startup` or `scoped`, and carry the moment they happened; every registered playbook is announced once the arena has announced itself, so a playbook that has not run yet is still visible
- `arena-dashboard`: playbook reports join the arena's own timeline, recorded per arena and served from `GET /history`, so the graph and the playbook panel replay and scrub off one history keyed by the arena's own timestamps
- `arena-dashboard`: a component's detail dialog carries the same cpu profile report button its graph node does, both from one shared control
- `examples`: the FastAPI example scores a reading on the event loop thread in pure Python, holding the interpreter lock for roughly a quarter of a second per request so it dominates the captured flamegraph
- `arena-dashboard`: a canvas whose server connection is lost says so, keeps showing the last known state, and offers replay for as far as it got
- `arena-dashboard`: the playbook panel stays on screen when an arena has no playbooks and says so
- `arena-dashboard`: each arena's canvas carries a scrollable playbook panel on its left, one row per playbook with its live state, faulted rows marked, and a row click opening the same detail dialog the graph nodes use
- `arena-dashboard`: the lifecycle demo simulates four parallel test runs per arena, and both arenas run theirs at once, each opening every scoped playbook (one of them caller-implemented) and dropping the handles at the end, the way a parallel suite would
- `arena-dashboard`: the lifecycle demo registers startup playbooks (`apply-schema-migrations`, `seed-reference-data`) and per-arena scoped ones, cycles the scoped ones while the arenas are open, renames its subjects onto an Alice in Wonderland theme, and gains a fourth arena whose startup playbook faults
- `arena`: publishing and observing are split into `Publisher<E>`, `Observer<E>` and an `EventBus<E>` that owns the routing, with symmetrical register, unregister and bounded flush on both sides; the arena lifecycle and cpu profile channels now share that one mechanism instead of hand-rolling it twice
- `arena`: `LifecycleRule` validates state transitions, `Monotonic` for the arena phase and `RunnableRule` for any restartable subject, so an illegal transition is logged as a defect rather than published
- `arena`: scoped playbooks report their own `RunnableState` on their own channel, `Starting` and `Started` from the playbook and `Stopping` and `Stopped` from the handle's drop, so a playbook that runs and stops repeatedly is observable; events carry the arena id, and a failing playbook's fault lands on its own row
- `arena-dashboard`: `POST /arena/{arena}/playbook/{playbook}` ingests scoped playbook state, keyed per arena so two arenas can run the same playbook id, and broadcasts it on the existing event stream; the forwarder now also logs a rejected update instead of discarding the response
- `arena-profile`: new crate sampling CPU profiles and reporting the captured folded stacks through its own observer
- `arena-profile`: `perf`, async-profiler, py-spy and dotnet-trace backends selected from the component's build tool
- `arena-profile`: `CpuProfileObserver` reports the profiler lifecycle as `RunnableState`, carrying the folded stacks on the `Stopped` transition
- `arena-profile`: `render_folded` returns the inferno SVG plus hotspots ranked by self time with severity bands
- `arena`: `RunnableComponent::attach_cpu_profiler` switches a component's profiler on, defaulting to a fault for component types that cannot be profiled
- `arena-executable-component`: implements `attach_cpu_profiler`, resolving a backend from the build tool and faulting when the tool cannot be profiled
- `arena-junit`, `arena-xunit`: `BuildTool.PYTHON`/`BuildTool.Python`
- async-profiler and py-spy vendored through Bazel, so no host install is required
- `arena-dashboard`: new service ingesting `ArenaLifecycleObserver` state over HTTP (`HttpForwardingObserver`) and streaming it live to a browser dependency-graph viewer
- `arena-dashboard`: replay a recorded arena lifecycle, scrub it with a timeline slider, and reset or stop the server from the UI
- `arena-dashboard`: server binary packaged into the release artifacts for linux-x86-64, darwin-aarch64 and darwin-x86-64
- `arena-pytest`: `ClosedArena.observe(callback)` registers a lifecycle-state observer for the open arena
- `arena-dashboard`: browser test driving the packaged server binary and real arena lifecycle events in Chrome
- `arena-dashboard`: unit test coverage for the lifecycle-forwarding observer, previously untested
- `arena-pytest`, `arena-junit`, `arena-xunit`: `with_lifecycle_dashboard()` returns a dashboard builder carrying `with_cpu_profiler(component_id)`, repeatable per component, with `done()` back to the arena
- `arena`: `Match` resolves the requested profiler component ids at open, faulting on an unknown or ambiguous id
- `arena-dashboard`: `POST /profile/cpu` ingests profiler state and folded stacks, `GET /profile/cpu/{component}` returns the rendered SVG with hotspots, and profiler state reaches the browser on the existing event stream
- `arena-ffi`: `arena_register_dashboard_observer` also subscribes the dashboard to profiler events, removed again with the lifecycle observer token
- `arena-profile`: captured cpu profiles are capped at 8 MiB by default and fault rather than truncate, with the breach reported through `CpuProfileError`
- `arena-pytest`, `arena-junit`, `arena-xunit`: `with_cpu_profile_limit_bytes(bytes)` on the dashboard builder raises the cpu profile cap
- `arena-executable-component`: a failed cpu profile capture faults the component's stop instead of only being logged
- `arena-dashboard`: the lifecycle demo binary reports a cpu profile lifecycle for two of its components, so the profiler node state and flamegraph can be driven without a real profiled process
- `arena-dashboard`: a profiled component's node shows the profiler's live state and opens its flamegraph in context over the graph, pinnable, with a ranked hotspot table, regex filter, hover linking, layout toggle and a full screen view at `/?profile=<component>`
- `arena-dashboard`: clicking a node or a playbook opens the detail in the same draggable, resizable, pinnable panel the cpu profile uses, at its previous size, and a playbook's detail names its type, kind and run count; pinned panels stay open when another node, playbook or profile is opened
- `arena-dashboard`: the header separates the server connection from the colour legend, each in its own labelled box, and the legend is grouped into `state` and `subject`, naming the states Arena itself reports (`not_started`, `registered`, `starting`, `readiness_check`, `started`, `running`, `stopping`, `stopped`, `faulted`)

### Changed

- `arena-profile`, `arena-executable-component`: profiler environment carries a per-variable merge rule, so `JAVA_TOOL_OPTIONS` is space-joined behind a caller's value while `PYTHONPATH` is path-joined with the platform separator; the previous single space-join would have destroyed a caller's `PYTHONPATH` and produced one unusable `sys.path` entry
- `arena-profile`: a sampler's `collect` returns the folded stacks rather than a path and takes the byte limit, so the directory holding them can be released once they are read
- `arena-profile`: perf, py-spy, dotnet-trace and async-profiler all write into a `ScratchDir` held by their session, replacing the predictable `temp_dir/prefix-pid-counter` naming, which `scratch_path` provided and which is now gone
- `arena-container`: `credential::new_password()` takes no prefix, generating the whole password itself
- Example suites name their language in the arena id so the dashboard can tell them apart: the Spring Boot suites became `JavaSpringBootComponentTestSuite` and `JavaSpringBootChainedComponentTestSuite`, and the FastAPI arenas became `example-fastapi-python-arena` and `example-fastapi-python-chained-arena`
- `arena`: `ArenaLifecyclePublisher::transition`, `notify_progress` and `finish` no longer take subject lists; a match reserves a `MatchSlot` and reports into it, and the publisher assembles the arena snapshot from those frames
- Arena dependency teardown runs in parallel instead of sequentially
- `zip` 2.4.2 -> 9.0.3 (RUSTSEC-2026-0367, RUSTSEC-2026-0368, RUSTSEC-2026-0369)
- `rustls` 0.23.43 -> 0.23.46 (RUSTSEC-2026-0285, RUSTSEC-2026-0346, RUSTSEC-2026-0349)
- Example apps: `anyio` 4.14.2 (CVE-2026-63374, CVE-2026-64847), `netty-handler` 4.1.137.Final (CVE-2026-75595, CVE-2026-75596)
- `pyjwt` 2.13.0 -> 2.15.1, `urllib3` 2.7.0 -> 2.8.0 (multiple PyJWT and urllib3 CVEs)
- `aws-smithy-json` 0.62.5 -> 0.62.7 (GHSA-8ffr-xgwf-xj56, stack overflow via recursive unknown-key skipping)
- Example Spring Boot app and the Maven bench tool: `jackson-databind`/`jackson-core` pinned to 2.21.7/2.22.3
- `at.yawk.lz4:lz4-java` 1.11.1 -> 1.11.4 (multiple CVEs, pulled in transitively via `kafka-clients`)
- CI: 24 duplicated Bazel setup and BuildBuddy blocks replaced with a `.github/actions/bazel-setup` composite action
- Unit test coverage raised across the dependency crates, `arena-junit`, `arena-xunit` and `arena-pytest`
- Agent rules forbid asserting on log output to prove behaviour, and require the changelog to record internal as well as external changes
- Known-unfixable or false-positive CVEs (Spring Framework SSE/XsltView, PyJWT's `decode()` options-reuse bypass, `braces`) documented and suppressed with risk rationale in `osv-scanner.toml` instead of silently ignored

### Fixed

- `arena-executable-component`: a wrapping profiler whose finish fails has its recorder killed before the component is reaped, where teardown could block forever with the recorder still holding the traced process
- `arena-profile`: a profile flushed before the component was torn down is read even when the process does not exit within the finish budget, where the previous ordering discarded it
- CI: the .NET predeploy smoke test takes only the FFI library from the staged prebuilt directory, instead of everything in it, so the dashboard server binary now staged alongside it no longer breaks the macOS NuGet smoke jobs
- `arena-profile`: a profile directory is removed on every exit path, including a teardown that times out before `collect` runs and a spawn that fails after the profiler was prepared, where the previous naming left files behind
- `arena-dashboard`: a per-arena history breach refuses the report instead of dropping the oldest, so a replay can no longer silently lose its first states
- `arena-postgres`, `arena-mssql`: a readiness timeout no longer puts the connection string, password included, in the fault message that reaches the dashboard and its history
- `arena-profile`: async-profiler writes to a pid-scoped file and the sampler reads back the profiled child's own output, so a launcher process can no longer overwrite the component's profile
- `arena-executable-component`: profiler environment is appended to a caller's existing value for the same variable instead of replacing it
- `examples`: the Java and .NET examples rename their second web app so its name no longer shares a profiler request with the first one
- `arena-profile`: the async-profiler agent travels in `JAVA_TOOL_OPTIONS` instead of being prepended to the component's arguments, so a Java component launched through a wrapper script is profiled instead of faulting at capture with no output file
- `arena-dashboard`: the hotspot table lays out in fixed columns with the function name truncating, so self and share stay visible without scrolling sideways
- `arena-dashboard`: the header is in the page flow instead of floating over it, so narrowing the window no longer hides the first arena's replay button and timeline behind the wrapped legend; the clear and stop buttons sit in the top right at the title's level and only the legend wraps
- `arena`: a match published only its own dependencies and components, so in a multi-match arena every frame it emitted dropped the other matches' subjects from the dashboard until the arena published again; the lifecycle publisher now holds one frame per match and assembles the whole arena from them
- `arena`: progress snapshots described a dependency or component still in flight with no children, faults or profiled component ids, so nested subjects vanished from the dashboard canvas mid-start and mid-teardown; the snapshot now carries what the subject last reported
- `arena`: `PlaybookLifecycle::exit` could underflow its live-handle count when a playbook built through the dependency-level builder was dropped without a matching run, which silently stopped that playbook ever reporting stopped again; a startup playbook no longer keeps a retained handle counted after it finishes, and a playbook whose teardown panics releases its handle instead of leaking it
- `arena`: a playbook run that fails now reports itself faulted from the base type rather than waiting for the arena to report, so a direct `run()` no longer leaves the row reading as live
- `arena`: a startup playbook now reports stopped as soon as its run returns, instead of staying started for the whole open arena because the arena holds its handle until teardown; the handle drop at teardown adds nothing further
- `arena-oracledb`: the default database and admin passwords are generated per run through `arena-container`'s existing unique-suffix generator instead of being literals in the source, which also clears the hard-coded-credential alert code scanning raised against them
- Agent rules forbid hard-coding a password, secret or key in any test or example; generate it per run the way `examples/tests/pytest-shared/arena_config.py` does
- `arena-http`: the loopback TLS verification alert from code scanning is dismissed with its reason recorded, and the real fix (Arena generating the HTTPS stub's own certificate) is tracked in GH #279
- `arena-ffi`: the dashboard observer tests no longer race each other through the process-wide observer registry; one test now covers register, forward and remove, and asserts on cpu profile posts rather than on any request reaching the listener
- `arena-dashboard`: `ServerShutdown::trigger` set the shutdown flag only while something was already subscribed, so a `POST /shutdown` arriving before the server subscribed was lost and the later wait never returned
- macOS release binaries are re-signed ad-hoc after stripping, so the darwin `arena_ffi_shared` no longer ships with an invalidated signature
- `OpenArena::run_playbook` now records a panicking playbook as a `Fault` instead of letting it unwind out of the arena
- Executable components no longer mis-resolve a bare command name (e.g. `powershell`, `sh`) against the working directory; PATH-resolved commands are left alone
- `arena-ffi`'s `HttpReadinessCheck` now requires a successful (2xx) status instead of treating any HTTP response, including 404/500, as ready
- `scripts/repin.py` no longer leaves `MODULE.bazel.lock` stale: `CARGO_BAZEL_REPIN` is scoped to the crate repin step and the lockfile is recorded last
- `requirements_windows.txt` for `examples` and `arena-pytest` regenerated to carry the same `pyjwt`/`urllib3` fixes as the other platform lockfiles
- Windows CI jobs no longer use a Bazel disk cache, working around upstream Bazel 7.4.1 bug bazelbuild/bazel#28408 where a concurrent rename into the disk cache CAS fails with "Permission denied"; the BuildBuddy remote cache still applies
- Bazel: the vendored profiler tool binaries are declared on the targets that run them instead of on the `arena-profile` library, keeping the .NET SDK out of the runfiles of every transitive consumer (a dependent Python test's runfiles drop from 6,533 to 2,644 files)
- Windows CI test jobs pass `--nobuild_python_zip` alongside the `--enable_runfiles` they already set, so Python tests run from the runfiles tree instead of building and self-extracting a zip of it; `//arena-pytest:arena_pytest_unit_test` and `//examples:ephemeral_test_runtime_test` no longer exceed the 60s `small` timeout on that overhead alone
- macOS CI test jobs no longer pass `--local_test_jobs=6`, which over-subscribed the 3-core and 4-core runners and starved tests; Bazel now sizes local test concurrency from `--jobs`
- A cpu profile that captured no samples is reported as a stopped profiler with no profile instead of faulting the arena, across the py-spy, perf, async-profiler and dotnet-trace backends; a genuine profiler failure still faults, and py-spy's now names its exit status
- The five cpu profiling component tests share one busy-loop subject per language under `//tools/profile_subjects`, each burning a known budget across ten named functions with one dominant hotspot, and every test now waits for the subject to complete a pass rather than sleeping for a guessed interval; this is the race that failed on arm64 CI
- `arena-dashboard`: the lost-connection banner is replaced by a plain `disconnected` notice, shown only while no arenas are held in memory, covered by a browser test that kills the server under an open page
- `arena-dashboard`: the arena graph refits once its nodes are measured, so subjects that stream in after the page is open are no longer left outside the clipped viewport
- `arena-dashboard`: the graph no longer swallows the wheel, so the page scrolls past stacked arenas; zoom moves to ctrl+wheel and to zoom in, zoom out and Recenter buttons on the canvas
- `arena`: `LifecycleContext` is renamed `ArenaLifecyclePublisher` and moved to `lifecycle/publisher.rs`, since it only ever publishes
- Maven predeploy smoke probe uses the dashboard builder's `open()` shortcut, fixing a compile failure introduced when `withLifecycleDashboard` started returning a builder
- `arena-dashboard`: `HttpForwardingObserver` observes all three event channels through small single-event forwarders rather than one type with three callbacks, and its drain is now an explicit `flush` that waits for the queue to empty within a budget, with `Drop` still owning shutdown
- `arena-dashboard`: an arena id starting a fresh run replaces the finished run's history and log instead of appending to it, so a second run no longer reads as the same arena starting and tearing down twice
- `arena-dashboard`: replay and the no-auto-replay guarantee are now covered by tests, the first coverage that feature has had
- `arena-dashboard`: a live-joined page draws its graph edges again; react-flow's `remove` changes are no longer applied to nodes and edges that are derived from the arena snapshot
- `arena-dashboard`: the demo's profiles model a real service under load, around 50 frames deep, with the dominant hotspot a blocking HTTP call made through `futures::executor::block_on` on a tokio worker, a regex compiled per request, and an N+1 query path
- `arena-dashboard`: the flamegraph's own zoom and search work again; inferno binds its handlers from `onload` on the `<svg>` root, which never fires for inline SVG, so the embedded document now calls its `init` once parsed
- `arena-dashboard`: the cpu profile panel has a grab bar that drags it, snaps to the nearest viewport corner on release, resizes from all four edges and corners, and uses a pin icon that reflects the pinned state; a profiled node widens to fit its profiler chip rather than clipping it
- `arena-dashboard`: the lifecycle demo profiles `orders-api` as well, and its faulted arena carries distinct subject ids so it cannot display a profile it never produced
- `arena-dashboard`: browser tests cover the real profiler workflow end to end (a demo component sampling, then its report opening as a flamegraph with its hotspot), the disconnect notice, and a stopped profiler that sampled nothing offering no report
- `arena-dashboard`: a profiled component's node shows `cpu profiler <state>` live as the profiler reports it, and only becomes a button once the server confirms a report is held for that component
- `arena-dashboard`: a profiler run that sampled nothing clears any profile stored for that component, so the dashboard cannot serve a previous run's flamegraph as the current one
- Agent rules: never let a sleep or timeout carry an assertion, wait on an observable condition bounded by a named deadline
- Agent rules: bug reports must lead with crate, type and function, then one sentence
- Agent rules: bound unbounded growth with a modest default cap the caller can raise, faulting when breached, rather than evicting or truncating
- Agent rules: re-read the AI communication section before every reply, and never write an essay
- Dropped three stale `osv-scanner.toml` ignores (one `@vitest/mocker`, two tinypool) now that vitest is 5.0.4 and tinypool has left the dependency tree

## [7.0.1]

### Fixed

- Subject-scoped records carrying no `dependency`/`component` field are matched against the log identifier lists by their span
- Playbook-scoped records reach registered log targets instead of being dropped by the dispatcher
- Faults and panics are logged where they occur, naming the subject and the cause chain
- Timing and phase records moved to DEBUG, leaving lifecycle transitions at INFO
- arena-pytest keeps its own frames out of pytest tracebacks

## [7.0.0]

### Added

- Arena lifecycle: a full state machine (`ArenaLifecycleState`, `RunnableState`) with recursive `Fault`s and an `ArenaState` snapshot covering every dependency, component, child and recorded fault
- Lifecycle observation: `ClosedArena::observe` in Rust and `arena_add_lifecycle_observer` / `arena_remove_lifecycle_observer` / `arena_state_json` over the FFI, streaming one state document per transition
- Python, Java and .NET clients: an `ArenaState` model, `ArenaLifecycleError` carrying the parsed state, a state accessor on the open arena, and identical lifecycle transition plus `closing summary` log lines
- Guaranteed teardown: a graceful stop followed by an unconditional, idempotent forced stop over every subject (`force_stop`, `release`), plus container expiry so crashed runs are cleaned up
- Object-namespaced logging: every line under `arena.<id>`, `arena.<id>.dependency.<id>` or `arena.<id>.component.<id>` with ` | ` separated fields

### Changed

- Every lifecycle API returns `Result` with `Fault`s instead of panicking: core dependency/component/playbook traits, open/close, resets, and the oauth, http, oracledb and executable-component builders
- FFI: `arena_open` and `arena_close` hand back the arena state document; a faulted open or close surfaces as a typed lifecycle error in all three clients instead of panic text
- Log output shape: `[arena::module]` prefixes removed, fields pipe separated, ids already in the logger namespace dropped from messages, durations rounded to three significant figures, default dispatcher logger renamed to `arena`
- CI: the OracleDB component test runs in its own job on the Linux runners

### Fixed

- No exit path reports success while a subject may still be running: faults during parallel starts, stops, forced teardown, observers and drop are all recorded and teardown always completes
- Panics no longer escape to client consoles; they are contained at the FFI boundary and recorded as faults in the arena state
- Container name collisions from six-character identifier segments, non-invariant lowercasing, and double-free/finalizer races in the .NET client
- `arena-oauth` hard reset teardown, TLS generation failures reported as faults, and the expired-container sweep throttled to once per module per minute

## [6.2.1]

### Changed

- `arena-oauth`: issuer signing keys are generated when the dependency starts instead of when it is built
- `rsa` and `num-bigint-dig` build with optimizations in unoptimized builds, so RSA key generation no longer dominates test time

### Fixed

- Dependencies and containerized components now request a platform the image actually publishes, falling back to `linux/amd64` for amd64-only images, so arenas start on arm64 hosts
- `arena-oauth`: the HTTPS readiness probe now verifies the server certificate Arena generated instead of accepting any certificate
- `arena-http`: the admin client only skips certificate verification for loopback hosts, and directs callers to `with_trusted_certificate_pem` otherwise
- Platform resolution no longer caches a failed registry lookup, keys the cache by host platform, and falls back to the platform of the locally present image when the registry cannot be reached
- Container start failures report the exit code as a plain number instead of Rust debug formatting
- `arena-oauth`: the ephemeral server certificate covers the configured listen address, so a non-default listen IP passes the readiness probe

## [6.2.0]

### Added

- `arena-oauth`: multi-issuer support on `OauthDependency` via `with_issuer`/`with_provider` (#218)
- `arena-oauth`: `Provider` presets for Cognito, Okta, and Entra ID (#218)
- `arena-oauth`: `sign_claims`/`signing_key_pem`/`issuer_at`/`issuer_count` on `OauthDependency` (#218)
- `arena-ffi`: `arena_oauth_sign_claims` (#218)
- `arena-pytest`, `arena-junit`, `arena-xunit`: `with_issuer_cognito`/`with_issuer_okta`/`with_issuer_entra_id`/`with_issuer`/`sign_claims` bindings (#218)
- Example apps (Rust, Spring Boot, ASP.NET): Cognito-shaped OAuth provider in test fixtures (#218)
- `arena-junit`: `OauthSigner`/`@ArenaOauthSigner` for injecting a per-test OAuth signer (#218)
- `arena-xunit`: `ArenaCollectionFixture.Signer`/`GetDependency<T>()` for injecting a per-fixture OAuth signer (#218)
- `arena-pytest`: `oauth_signer_fixture` for wiring an `OauthSigner` pytest fixture (#218)
- `arena-host`: new crate providing `find_available_port`/`PortSearchStrategy` for zero-dependency, process-safe free TCP port discovery (#202)
- `arena-ffi`: `arena_find_available_port` (#202)
- `arena-pytest`, `arena-junit`, `arena-xunit`: `find_available_port`/`ArenaHost` bindings, `PortSearchStrategy`, `ArenaPortNotFoundError`/`ArenaPortNotFoundException` (#202)
- `--config=stream` for streamed test output with a detailed summary
- mold linker for Rust targets on Linux, installed by CI and required for local builds

### Changed

- `bazel test` now defaults to `--test_output=errors`; use `--config=stream` for streamed output

### Fixed

- `arena-junit`: `ArenaExtension` now makes the arena queryable during `@ArenaAfterOpen`, not just after
- `arena-oracledb`: SQL readiness fails immediately when the container has stopped or been removed, instead of retrying it for the full timeout
- FastAPI example component tests: dependency containers get run-unique names, so parallel targets no longer force-remove each other's containers
- Example test runtimes (pytest, junit, xunit): ephemeral port ranges partitioned per test target to stop parallel targets drawing colliding host ports (#220)

## [6.1.0]

### Added

- `audit_and_vet_rust` now fails if the count of cargo-vet audited or exempted packages changes, to catch silent supply-chain trust regressions (or acknowledge improvements)
- arena-junit: Oracle dependency and playbook support (`OracleDependencyBuilder`, `ManagedOraclePlaybook`)
- `arena-xunit`: Oracle dependency and playbook support (`OracleDependencyBuilder`, `ManagedOraclePlaybook`)
- `arena-pytest`: Oracle dependency and playbook support (`OracleDependencyBuilder`, `ManagedOraclePlaybook`)
- FastAPI example app: Oracle-backed weather report endpoints (`POST/GET /weather`), wired into both pytest component test suites
- Spring Boot and ASP.NET example apps: Oracle-backed weather report endpoints, wired into both junit and xunit component/chained-component test suites
- Oracle dependency builders (`arena-oracledb`, `arena-junit`, `arena-xunit`, `arena-pytest`) require an explicit `.full_build()` opt-in for a non-default database name, and scale the SQL readiness timeout accordingly, since a custom name forces a slow from-scratch pluggable database build
- CI: betterleaks secret scan on every PR, push to master, and daily cron

### Fixed

- `fromImage` now resolves registry credentials (`credHelpers`, `credsStore`, `auths`, honoring `DOCKER_CONFIG`) instead of pulling anonymously (#210)
- `fromImage` skips pulling when a locally cached image already matches the requested platform (#210)
- Image pull/build failures now return a typed error instead of panicking (#210)
- Registry credential resolution no longer blocks the async runtime and no longer silently swallows credential-helper failures (#210)
- Example Postgres/MSSQL test credentials are now randomly generated instead of hardcoded, matching the existing Oracle credential pattern

### Changed

- PR pipeline no longer auto-publishes preview builds (TestPyPI, Maven snapshot, NuGet snapshot) on every push; apply the `pre-release` label to opt in

## [6.0.2]

### Security

- Bumped `h2` to 0.4.16, fixing RUSTSEC-2026-0258
- Bumped `jackson-databind`, `grpc-core`/`grpc-netty-shaded`, `netty-codec`/`-http`/`-http2`, and `log4j-api` in the example Spring Boot app (`pom.xml` and `MODULE.bazel`) to patched versions
- Pinned `arena_java_maven` to a checksum-verified lock file (`arena_java_maven_install.json`) instead of unverified live resolution
- Enabled hash verification (`generate_hashes = True`) for `arena-pytest` and `examples` pip lockfiles
- Added floor constraints (`idna>=3.15`, `pygments>=2.20.0`) to `arena-pytest`/`examples` `requirements.txt` so OSV-Scanner's independent resolution of the unpinned manifest also picks up patched versions
- Documented OSV-Scanner exceptions (`osv-scanner.toml`) for `rsa`, `rustls-pemfile`, and `rustls-webpki` advisories with no reachable fix through the `tiberius` dependency chain
- Extended OSV-Scanner CI coverage across all 4 dependency ecosystems (Cargo, pip, Maven, NuGet) by adding generated CycloneDX SBOMs for `arena-ffi` (Rust), `arena-junit` (Maven), and `arena-xunit` (NuGet); pip was already natively scanned via `requirements_lock.txt`. Closes a gap where the actual published Maven/NuGet dependencies (as opposed to the example apps) had no CVE scanning at all
- Bumped `kafka-clients` to 3.9.2, migrated `org.lz4:lz4-java` to its relocated `at.yawk.lz4:lz4-java` coordinates at 1.11.1, and bumped `micrometer-core` to 1.15.12, fixing CVEs surfaced by the new Maven SBOM scan
- Closed a fail-open gap in `check_dependency_release_age.py` where Maven/BCR/NuGet dependencies silently passed the 3-day release-age check if their publish time couldn't be determined; also replaced the unreliable Maven Central search-index and empty BCR-timestamp lookups with direct repository/commit-history queries
- Bumped the pinned `google/osv-scanner-action` from a May 2026 commit to v2.5.1, picking up native `.csproj` (NuGet) scanning support that the old pin lacked

### Added

- `bazel run //scripts:repin` to force-repin Rust, Python, and Maven lockfiles independent of a version bump
- `scripts/generate_rust_sbom.py`, `generate_maven_sbom.py`, `generate_nuget_sbom.py` to produce CycloneDX SBOMs for OSV-Scanner from `Cargo.Bazel.lock`, `arena_java_maven_install.json`, and `MODULE.bazel`
- `cargo audit bin` and `cargo vet` against a `cargo-auditable`-built `arena_ffi_shared` binary, run as part of `bazel run //scripts:repin` and as a standalone, Rust-path-filtered CI job (`supply-chain/` cargo-vet config, bootstrapped with exemptions for all currently-used crates)
- Added `chrono` as a direct workspace dependency (with the `clock` feature) so `arena-kafka`/`examples` no longer rely on `rskafka`'s re-exported `chrono` having that feature enabled only by incidental cross-crate feature unification; this is what made a standalone `cargo build -p arena-ffi` possible in the first place

### Fixed

- Example Axum web app now retries the Postgres connect and OAuth JWKS fetch on startup instead of panicking on the first attempt, fixing flaky example component tests under CI load

### Changed

- Bumped `rustls` to 0.23.43

## [6.0.1]

### Fixed

- `arena-xunit` native library resolution now prefers a co-located `libarena_ffi_shared` over `.deps.json`/`AssemblyDependencyResolver`, fixing wrong-RID resolution on Apple Silicon

### Added

- NuGet predeploy smoke test now runs via a real xUnit test project (`dotnet test`) against the real global NuGet cache, not an isolated one
- New Bazel-consumer predeploy smoke test (`//tools/predeploy_smoke/dotnet:bazel_consumer_smoke_test`) covers the `rules_dotnet csharp_import` + `bazel test` consumption path

## [6.0.0]

### Added

- `arena-xunit` API parity with `arena-pytest`/`arena-junit`: `WithImageName`/`WithImage`/`WithContainerName` on Kafka, Mssql, Postgres, and Smtp dependencies; `WithLambda` on Localstack; `WithHttp()`/`transport` and default-issuer fallback on Oauth; `WithBuildToolCustom` on `ExecutableComponentBuilder`

### Fixed

- `OauthDependencyBuilder` in `arena-xunit` defaulted to port 9443 instead of 9444, matching `arena-pytest`/`arena-junit`
- `KafkaDependencyBuilder.Build()` in `arena-xunit` aliased its internal topic list instead of copying it, so mutating the builder after `Build()` mutated the already-built dependency
- `LocalstackDependencyBuilder.WithLambda` source directory now expands a leading `~` before resolving to an absolute path

### Removed

- `LocalstackDependencyBuilder.WithImage(string)` in `arena-xunit`: sent a wire key the Rust FFI never read, making it a silent no-op; replaced with `WithImageName`/`WithImageTag`

## [5.4.1]

### Fixed

- `arena-xunit` NuGet package placed native libraries under `lib/<tfm>/` instead of `runtimes/<rid>/native/`, so they were never deployed to consumers; `ArenaPaths` also gained an `AssemblyDependencyResolver` fallback for the common case where no `RuntimeIdentifier` is set

### Added

- Predeploy smoke test gates (`tools/predeploy_smoke/`) for arena-xunit, arena-junit, and arena-pytest: each publish job now installs the real built artifact via its real package manager and opens/closes an arena before publishing, with cross-platform execution coverage on macOS and a fast hermetic packaging-layout test covering all RIDs/classifiers including Windows

## [5.4.0]

### Added

- `ContainerizedComponentBuilder.fromImage()` to run an already-published registry image instead of building from a Containerfile, `withPlatform()` to override the container platform, and validation that rejects `withImageTag()`/`withBuildContext()` combined with `fromImage()` (#198)

## [5.3.1]

### Added

- Developer Certificate of Origin (DCO) sign-off requirement and enforcement workflow

### Security

- Bumped `jackson-databind` to 2.22.2 in `arena-junit` (transitive resolution was picking 2.21.4, vulnerable to GHSA-5gvw-p9qm-jgwh, GHSA-5jmj-h7xm-6q6v, GHSA-mhm7-754m-9p8w)

## [5.3.0]

### Added

- Bind-mount (volume) support for `ContainerizedComponentBuilder` across Rust, `arena-pytest`, `arena-junit`, and `arena-xunit` (#185)

## [5.2.0]

### Added

- Windows (`x86_64-pc-windows-msvc`) support for `arena-ffi`, and native binary packaging for `arena-pytest` (PyPI), `arena-junit` (Maven), and `arena-xunit` (NuGet)

## [5.1.3]

### Changed

- Replaced `rdkafka` with `rskafka` in `arena-kafka` (pure Rust, no C toolchain required)

## [5.1.2]

### Added

- Additional unit test coverage across `arena-junit`

## [5.1.1]

### Added

- Additional unit test coverage across `arena`, `arena-http`, `arena-kafka`, `arena-postgres`, `arena-localstack`, `arena-oauth`, and `arena-mssql`

## [5.1.0]

### Added

- Dependency and component trees: `with_child_dependencies`/`with_child_components` (Rust: `add_child`) across Rust, Java, Python, and .NET clients
- ClusterFuzzLite continuous fuzzing (`fuzz/`, `.clusterfuzzlite/`)

### Fixed

- xunit ASP.NET example app: health check now waits for the Temporal worker to be actively polling before reporting ready, fixing flaky startup/logging in the xunit example tests
- Managed playbooks and `arena_soft_reset`/`arena_hard_reset` now resolve dependencies nested as children, not just top-level dependencies
- arena-ffi entry points no longer crash the host process on malformed JSON input; panics are now returned as errors
- arena-pytest: `ManagedPostgresPlaybook` is now exported from the package root
- Documented accepted `cargo audit` advisories for `rsa`/`rustls-webpki` transitive deps (no upstream fix available; low exploitability for Arena's ephemeral local connections)

## [5.0.0]

### Breaking

- Scoped `Managed*Playbook` activation now runs after the test body by default instead of before (Python, Java, .NET); custom Python `Playbook` subclasses must now extend `ManagedPlaybook` or `UnmanagedPlaybook`

### Added

- Unmanaged playbook support in arena-pytest, arena-junit, arena-xunit
- arena-junit: share one arena across multiple test classes via `@Suite`/`@SelectClasses` and `@Arena(value = ...)`
- `--bump major/minor/patch` flag for the version bump script

### Changed

- arena-xunit `ManagedPlaybook.Run()` is now overridable
- arena-pytest and arena-xunit published packages use the root `README.md` instead of per-package copies

### Fixed

- arena-junit `ActivePlaybook` no longer rejects a zero/null handle
- Bump Spring Boot 3.5.3 → 3.5.16, `jackson-databind` → 2.18.9, `logback-classic` → 1.5.38 (CVE fixes)

### Removed

- arena-pytest and arena-xunit per-package `README.md`/`DESCRIPTION.md` files

## [4.0.1]

### Fixed

- Bump `cryptography` 48.0.1 → 50.0.0 in examples (CVE fix)

## [4.0.0]

Not published to PyPI, Maven Central, or NuGet due to a GitHub Actions outage; superseded immediately by 4.0.1.

### Changed

- Reduced release build size; split the release build matrix per platform for arena-junit
- Release builds now run on every merge to master

### Fixed

- Rust builds on Intel Mac (missing `x86_64-apple-darwin` in `crate.from_cargo` `supported_platform_triples`)
- Maven Central publish pipeline

## [3.7.0]

### Added

- Scoped `@ArenaLogger`-style logging annotation for arena-junit and arena-xunit's `ArenaCollectionFixture`
- FFI test coverage for postgres and Temporal dependency wiring

### Fixed

- Dependency log forwarding gaps

## [3.6.0]

### Added

- `ManagedPostgresPlaybook` across arena-pytest, arena-junit, and arena-xunit, with FFI bindings and a Postgres playbook manifest

## [3.5.9]

### Fixed

- Release publish pipeline (CI workflow, `.bazelrc`, lockfile sync)

## [3.5.7]

### Fixed

- Native library resolution/packaging for arena-junit and arena-xunit (`ArenaBindings`, `ArenaNativeLib`, `ArenaPaths`)

## [3.5.6]

### Changed

- arena-junit, arena-pytest, and arena-xunit package description metadata (POM, `DESCRIPTION.md`, nuspec)

## [3.5.5]

### Fixed

- Renamed the arena-xunit NuGet assembly/package for publishing

## [3.5.4]

### Changed

- Dependency lockfile repin; CI workflow tweak

## [3.5.3]

### Added

- Publish arena-xunit to NuGet, including the `csharp_nuget_package` Bazel packaging toolchain (nuspec template, push script, pinned-version checks)

## [3.5.2]

### Changed

- Default container image CVE scan is now a label-gated GitHub Actions option instead of always running

## [3.5.1]

### Added

- TCP readiness check alongside the existing HTTP readiness check

## [3.5.0]

### Added

- STARTTLS support for arena-smtp (`with_starttls()` / `.withStarttls()`), backed by an ephemeral self-signed certificate shared via arena-container

## [3.4.0]

### Added

- arena-smtp mail-capture dependency crate, wired into arena-pytest (`SmtpDependencyBuilder`) and arena-junit (`SmtpDependency`)

## [3.3.0]

### Added

- Temporal dependency support in arena-pytest and arena-junit, with FFI bindings

## [3.2.1]

### Fixed

- Bump jackson-databind, postgresql, and mssql-jdbc to patched versions (CVE fixes)

## [3.2.0]

### Added

- `arena-temporal` dependency crate with a gRPC-based readiness check and pinned image tag

## [3.1.1]

### Changed

- Enabled BuildBuddy remote build caching for CI

## [3.1.0]

### Added

- Best-effort container image CVE search (`scripts/check_container_cves.py`) and bumped default image versions

## [3.0.1]

### Fixed

- Bump `serde_with` 3.18.0 → 3.21.0

## [3.0.0]

### Breaking

- arena-junit now targets Java 25

### Removed

- The unrelated `arbiter` crate and its tests

## [2.0.0]

### Breaking

- arena-junit examples move from a separate `ArenaFixture` class to declaring `@ArenaDependency`/`@ArenaComponent` directly on the test class (the `@Arena` annotation pattern)

## [1.2.4]

### Fixed

- Maven Central publish CI workflow

## [1.2.3]

### Fixed

- Maven Central publish script

## [1.2.2]

### Added

- Publish arena-junit to Maven Central, including the publish script and POM template

## [1.2.1]

### Added

- Dependency review workflow

### Changed

- Pinned default container image tags/versions with a security scan check

## [1.2.0]

### Added

- Container image default/version management scripts (`container_defaults.py`, image matrix, default-images codegen)

## [1.1.1]

### Fixed

- Bump `cmov` 0.5.3 → 0.5.4

## [1.1.0]

### Changed

- Simplified CI/CD: publish now triggers on version update instead of PR labels; removed the PR-label version-bump feature

## [1.0.0]

### Added

- First stable release
- `tools/version_sync.bzl` and related scripts to keep the workspace version in sync

### Changed

- CI now enforces a 3-day dependency release-age gate and locks down `MODULE.bazel.lock`

## [0.4.0-b1] (pre-release)

### Added

- Initial PyPI pre-release
