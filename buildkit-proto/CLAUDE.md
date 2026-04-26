# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Workspace structure

This crate lives inside a Cargo workspace rooted one level up (`../`). The workspace contains three crates:

- `buildkit-proto` — low-level protobuf bindings for BuildKit (this crate). Not meant to be used directly by end users.
- `buildkit-llb` — high-level API for building LLB (Low-Level Builder) graphs. Depends on `buildkit-proto`.
- `buildkit-frontend` — utilities for implementing BuildKit frontends. Depends on both above.

## Commands

Run from the workspace root (`../`) or within this crate:

```sh
cargo build                          # build all crates
cargo test                           # run all tests
cargo test -p buildkit-proto         # test this crate only
cargo test <test_name>               # run a single test by name
cargo fmt --all -- --check           # check formatting (CI gate)
cargo clippy --all-features          # lint (CI gate)
```

## Proto code generation

Protos are vendored under `proto/` and compiled at build time via `prost_build` in `build.rs`. Generated Rust files land in `OUT_DIR` and are pulled into the crate via `include!` macros in `src/lib.rs`.

To update vendored protos to a new BuildKit version, edit `BUILDKIT_VERSION` in `update.sh` and run it:

```sh
./update.sh
```

This downloads `.proto` files from upstream moby/buildkit, googleapis, and related repos.

## Module layout in `src/lib.rs`

The `include!` macros expose generated modules under these paths:

| Rust path | Proto package |
|-----------|---------------|
| `moby::buildkit::v1::frontend` | gateway/bridge gRPC service |
| `moby::buildkit::v1::apicaps` | API capabilities |
| `moby::buildkit::v1::types` | worker types |
| `moby::buildkit::v1::sourcepolicy` | source policy |
| `google::rpc` | Google RPC status |
| `pb` | solver ops (LLB graph nodes) |
| `fsutil::types` | filesystem stat types |
