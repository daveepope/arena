# Contributing to Arena

Contributions are welcome and encouraged. This document covers the basics for opening an issue or pull request.

## Getting started

- Bazel is the source of truth for builds and tests. Install [Bazelisk](https://github.com/bazelbuild/bazelisk).
- Docker is required for component tests.
- Build: `bazel build //...`
- Test: `bazel test //...`
- Do not run `pip`, `npm`, `maven`, or `nuget` directly on the host. Use the Bazel targets (e.g. `bazel run //arena-pytest:pip_requirements.update`).

See the [README](README.md) for an overview of the project and its clients.

## Branch naming

Use a prefix that describes the kind of change, followed by a short, hyphenated description:

- `feature/name-of-feature`
- `bug/name-of-bug`
- `chore/name-of-chore`

## Making changes

- Keep pull requests small and focused where applicable. One logical change per PR is easier to review and revert.
- No drive-by refactors or unrelated file changes. If you spot something unrelated worth fixing, open a separate issue or PR.
- Follow the conventions already established in the module you're touching (naming, layout, test structure). See [AGENTS.md](AGENTS.md) for the full set of project rules.
- Do not add new markdown or documentation files unless the change calls for it.
- If your change adds a new third-party dependency (any language), call it out explicitly in the PR description. Ensure the 3rd party dependency is absolutely necceasry.

## AI-assisted contributions

It's okay to use AI to assist in development, but do not submit PRs that are entirely AI-generated without review. These tend to contain incoherent changes, unnecessary abstractions, or outright hallucinations ("AI slop"). You are responsible for understanding and standing behind every line you submit. See [AI.md](AI.md): this project may not be used to train AI models.

## Keeping the client API surface consistent

Arena's public Rust API is the contract that the FFI layer and every client (Python, Java, .NET) build on. If you change it:

- Update the FFI layer to match, then update all client libraries that expose the changed surface, not just the one you happen to be using.
- The public API (method names, parameters, behavior) should read the same across languages, differing only in idiomatic casing (e.g. `with_port` in Rust, `withPort` in Java, `WithPort` in C#).
- Do not let one client gain a capability or option that the others silently lack. If a change can't be made consistently across all clients in the same PR, say so explicitly and flag which clients still need it.
- Verify with a build and test pass across the FFI and client targets affected, not just the Rust crate you changed.

## Don't leak implementation details into the public API

- Exported types, traits, methods, builder options, and user-visible error/panic strings should not expose which concrete tool backs a dependency. Callers shouldn't need to know or care that a swap happened under the hood.
- Use neutral naming (see [AGENTS.md](AGENTS.md) for the specific banned terms and naming rules). Do not carry vendor-, engine-, or tool-specific names into public symbols.
- The FFI layer stays language-independent: no Rust-specific idioms, no client-library concerns, no dependency-crate internals crossing the boundary.
- If you're unsure whether something belongs on the public surface, default to keeping it private and only widen visibility when a caller genuinely needs it.

## Tests and coverage

- Add tests for new code. New code should be covered at at least 80%.
- Prefer unit tests when the behavior under test doesn't require a running dependency. Reserve component tests (tagged `component_test`) for behavior that genuinely needs one.
- Component tests must be fast (seconds, not tens of seconds). A slow or flaky test is a defect to fix, not something to paper over with retries or longer timeouts.
- Run the relevant tests locally before opening a PR: `bazel test //...` (or scope to the affected package).

## Changelog

- Add an entry to [CHANGELOG.md](CHANGELOG.md) under the `[Unreleased]` section (or create one if it doesn't exist) for any user-facing change.
- Keep entries short: one line per change, no prose explanations, no restating the diff.

## Versioning

Arena follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html). Bump the version using the Bazel task rather than editing the `VERSION` file by hand:

```bash
bazel run //scripts:bump_version -- --bump patch   # or minor / major

Agent instructions

- AGENTS.md is the source of truth for AI/editor rules. CLAUDE.md and .cursor/rules/arena-agent.mdc are generated from it; do not edit them by hand.
- If you edit AGENTS.md, run bazel run //scripts:sync_agent_rules and commit AGENTS.md, CLAUDE.md, and .cursor/rules/arena-agent.mdc together.

Submitting a pull request

- Ensure bazel build //... and bazel test //... pass locally before submitting.
- Describe what changed and why in the PR description.
- Link any related issue.
