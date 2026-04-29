# Agent Instructions for Chronos

## Project Context

Chronos is an open source Rust project. It listens for messages on a Kafka input topic, stores delayed messages in PostgreSQL, and publishes those messages to a Kafka output topic at a later time. More project background, design notes, and runtime details are in [README.md](README.md).

This is an existing project. Agents must preserve the project's current structure, style, testing practices, and conventions unless the user explicitly asks for a larger change.

The active working branch for current work is `feat/prom_metrics`.

## Working Principles

- Optimize for a clear action trail. Future agents may start with no conversation history, so decisions must be recoverable from files, commits, and command output summaries.
- Document material changes when making them. At minimum, the commit message must explain the intent, relevant implementation notes, and verification performed.
- Keep edits scoped to the requested change. Do not reformat unrelated files or rewrite working code for style only.
- Do not discard or revert user changes. If the worktree has unrelated modifications, leave them alone.
- Prefer existing module boundaries and patterns over new abstractions.
- Update README, How-to notes, examples, or this file when behavior, setup, tests, or agent workflow expectations change.

## Rust Conventions

- This is a Cargo workspace with these members:
  - `chronos_bin`: main Chronos binary and library code.
  - `pg_mig`: PostgreSQL migration binary.
  - `examples/*`: example clients and utilities.
- The Rust toolchain is pinned in [rust-toolchain.toml](rust-toolchain.toml). Use that version unless the user asks to change it.
- Formatting is controlled by [rustfmt.toml](rustfmt.toml): 4-space tabs, `max_width = 160`, Unix newlines.
- Keep tests close to the code under `#[cfg(test)] mod tests` when following the existing unit-test style.
- Prefer typed Rust APIs and project helpers over ad hoc parsing or shelling out from Rust code.
- Preserve the project's async style based on Tokio, Kafka, PostgreSQL, tracing, and Prometheus metrics crates already in use.

## Verification Commands

Use the repository's Make targets and scripts as the source of truth.

- Default pre-commit verification:

  ```sh
  sh scripts/pre-commit-checks.sh
  ```

  This runs:

  ```sh
  make withenv RECIPE=lint
  make withenv RECIPE=test.unit
  ```

- Lint-only check:

  ```sh
  make withenv RECIPE=lint
  ```

  This runs `cargo check`, `cargo fmt -- --check`, and `cargo clippy --all-targets`.

- Unit tests:

  ```sh
  make withenv RECIPE=test.unit
  ```

  This runs `cargo test`.

- Build:

  ```sh
  make build
  ```

  This runs `cargo build`.

- Metrics/integration verification:

  ```sh
  make integration
  ```

  This starts Docker-backed PostgreSQL and Kafka dependencies, runs migrations, starts Chronos, publishes a test message, verifies delivery, and checks the Prometheus `/metrics` endpoint.

- Stop integration services:

  ```sh
  make integration.down
  ```

Run the narrowest useful checks while iterating, then run the default pre-commit verification before committing. Run `make integration` for changes touching Kafka/PostgreSQL behavior, runtime wiring, Docker setup, migrations, metrics exposure, or end-to-end message flow.

If a verification command cannot be run, document the reason in the final response and in the commit message.

## Commit and Push Policy

Agents should commit and push their changes unless the user explicitly says not to.

Commit messages must include a footer named `Model-version` containing the model that generated the commit. Example:

```text
docs: add agent workflow guidance

Document Chronos project conventions, verification commands, and agent
handoff expectations.

Verification:
- sh scripts/pre-commit-checks.sh

Model-version: GPT-5
```

Use concise subject lines that match the existing repository style, such as `feat(...)`, `fix(...)`, `docs:`, or `chore:`. Include enough body detail for a future agent to understand why the change was made and what was verified.

## Paper Trail Expectations

For each non-trivial change, leave evidence in one or more of these places:

- Code comments only where they clarify non-obvious behavior.
- Tests that encode behavioral expectations.
- Documentation updates for changed workflows, configuration, metrics, or operational behavior.
- Commit message body with the reasoning and verification.
- Final response summarizing changed files and checks run.

When making tradeoffs, record the chosen path and the reason. Avoid relying on chat history as the only explanation.

## Project-Specific Notes

- Chronos treats Kafka message bodies opaquely and forwards messages after delay; avoid adding application-level assumptions about payload shape.
- The README describes at-least-once delivery semantics. Preserve behavior that supports persistence, recovery from suspected node failure, and duplicate-safe processing.
- Metrics work on the `feat/prom_metrics` branch currently includes a Prometheus endpoint and metric-family checks in the integration script. Changes to metrics should preserve unit tests for registry output and integration checks for expected metric families.
- Local development commonly uses `.env` copied from [.env.example](.env.example) through `make withenv`.
- Docker Compose is used for local PostgreSQL, Kafka, Jaeger, and OpenTelemetry dependencies.
