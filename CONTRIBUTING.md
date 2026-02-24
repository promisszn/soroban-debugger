# Contributing to Soroban Debugger

Thanks for your interest in contributing. This guide explains how to set up a dev environment from scratch, run tests, follow code style, and submit changes.

## Table of Contents

1. Development Environment Setup
2. Project Setup
3. Running Tests
4. Code Style and Quality
5. Commit Message Conventions
6. Claiming and Working on Issues
7. Pull Request Process

## Development Environment Setup

### Prerequisites

- Git
- Rust (stable toolchain)

### Install Rust (from scratch)

We recommend `rustup` to manage toolchains.

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

Install the stable toolchain plus formatter and lints:

```bash
rustup toolchain install stable
rustup default stable
rustup component add rustfmt clippy
```

Verify:

```bash
rustc --version
cargo --version
```

## Project Setup

1. Fork the repository on GitHub.
2. Clone your fork:

```bash
git clone https://github.com/<your-username>/soroban-debugger.git
cd soroban-debugger
```

3. Add the upstream remote:

```bash
git remote add upstream https://github.com/Timi16/soroban-debugger.git
```

4. Create a branch for your work:

```bash
git checkout -b feat/short-description
```

5. Build once to ensure the toolchain is working:

```bash
cargo build
```

## Running Tests

CI runs the full workspace test suite with all features enabled. Match that locally:

```bash
cargo test --workspace --all-features
```

Run a single test by name:

```bash
cargo test <test_name>
```

Run a specific integration test file:

```bash
cargo test --test <test_file>
```

## Code Style and Quality

We follow standard Rust tooling and treat warnings as errors in CI.

Format:

```bash
cargo fmt --all
```

Check formatting (CI uses this):

```bash
cargo fmt --all -- --check
```

Lint (CI uses this):

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Guidelines:

- Use `rustfmt` for formatting and do not hand-format.
- Fix all `clippy` warnings.
- Prefer small, focused functions and clear error messages.
- Add tests for new behavior and bug fixes.
- Keep public APIs documented with Rust doc comments (`///`).

## Commit Message Conventions

We use **Conventional Commits** (see `cliff.toml`). Format:

```
<type>(optional scope): short summary

[optional body]

[optional footer(s)]
```

Common types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `chore`.

Examples:

```
feat: add ledger snapshot inspector
fix(cli): handle empty args in parser
docs: add CONTRIBUTING guide
refactor(runtime): simplify wasm loader
```

Tips:

- Use the imperative mood ("add", "fix", "update").
- Reference issues in the footer when applicable (e.g., `Closes #123`).

## Claiming and Working on Issues

- Check the issue tracker for open issues and labels like `good first issue` or `help wanted`.
- Before starting, comment on the issue to say you want to work on it.
- If an issue is already assigned, coordinate in the thread before beginning work.
- Keep one issue per PR when possible, and link the PR to the issue.

## Pull Request Process

1. Sync with upstream before finalizing your branch:

```bash
git fetch upstream
git rebase upstream/main
```

2. Ensure all checks pass locally:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

3. Push your branch and open a PR against `main`.
4. Include:

- A clear description of the change and motivation.
- The related issue number (e.g., `Closes #123`).
- Test results (commands you ran).

5. Address review feedback promptly. PRs are merged after approval and CI passes.

Thanks for contributing!
