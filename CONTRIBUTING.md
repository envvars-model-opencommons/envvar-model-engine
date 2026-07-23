# Contributing

## The easiest contribution

Fill in a field. Any declaration with `reviewed: None` or an empty `summary` is an
open invitation: add what you know, stamp `reviewed` with today's date, done. These
are one-line pull requests and they are the whole point of the model being
partial-friendly.

## Building

```sh
cargo build --workspace --all-features
cargo test  --workspace --all-features
cargo fmt --all -- --check
cargo clippy --workspace --all-features --all-targets -- -D warnings

# the third-party schema check CI runs — the same file, not a copy of it
cargo run -q -p argenv --example consumer > /tmp/example.json
python3 dev/conformance.py /tmp/example.json api/v1/example.json
```

## The test suite is the specification

Each file states one part of the contract, and each test name is a sentence about
how the model behaves. Reading them in this order explains the whole project:

| File | States |
|---|---|
| `crates/argenv/tests/scenario.rs` | The whole life of a contract, end to end. **Start here.** |
| `crates/argenv/tests/declaration.rs` | What makes a declaration well formed. |
| `crates/argenv/tests/reading.rs` | How a value is read, and from where. |
| `crates/argenv/tests/linting.rs` | What checking an environment catches — and deliberately ignores. |
| `crates/argenv/tests/document.rs` | The portable projection, its round trip, and forward compatibility. |
| `crates/argenv/tests/schema.rs` | What the emitted JSON Schema promises. |
| `crates/argenv-cli/tests/cli.rs` | The tool, and whether the committed artifacts are current. |

Shared fixtures live in `crates/argenv/tests/common/mod.rs`: one small,
complete interface that every file tests against.

A new rule lands with a test named after it. If a test name would not make sense
as a sentence in the README, the behaviour probably needs rethinking rather than
documenting.

## Regenerating the schema

The schema is generated. Never edit `schema/*.json` by hand:

```sh
cargo run -p argenv-cli -- schema -o schema/argenv-contract.v1.schema.json
cp schema/argenv-contract.v1.schema.json api/v1/contract.schema.json
```

`cargo test` fails if the committed schema is not the one the model produces, so
a hand-edited or stale schema is caught before review.

## Changing the model's shape

Adding, renaming, or retyping a `Record` field is a **contract change**:

1. Open an issue describing the field and why the model needs it.
2. Additive changes (a new optional field) keep `CONTRACT_VERSION`.
3. Breaking changes bump `CONTRACT_VERSION` and require a migration note.
4. Update `FIELD_DOCS` in `contract.rs` — the build panics if a documented field
   does not exist, which is intentional.

## Rules the code follows

- No naked primitive for a domain-meaningful value. Closed set → enum; structured
  scalar → validated newtype; reference → compile-checked. `String` only for prose.
- Anything a human would forget is derived. `reviewed` is the sole hand-written
  date, because it encodes human judgement.
- Every public item carries a doc comment (`#![warn(missing_docs)]`).
- New invariants land with a test in `tests/contract.rs`.
