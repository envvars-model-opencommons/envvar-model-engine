# Dev environment

An isolated, pinned toolchain and editor for working on argenv.

It lives here rather than at the repository root on purpose: argenv is published
to crates.io, not as a Nix module, and a root `flake.nix` would be mistaken for
packaging of the project itself. Nothing here builds argenv — cargo does that.

## Use

```sh
bash initdirenv.sh     # once after cloning: locks, allows, and builds
code .                 # the isolated editor  (or: code-dev .)
```

`.envrc` points direnv at this directory, so the shell loads whenever you enter
the repository.

## What is in the shell

| | |
|---|---|
| `rustc` `cargo` `rustfmt` `clippy` `rust-analyzer` | the same set CI uses |
| `python3` with `jsonschema` | run the schema conformance check locally |
| `jq` | read the generated schema and example documents |
| `cargo-llvm-cov` + `coverage` | line coverage, written where the editor reads it |
| `nil` `nixpkgs-fmt` | for editing this flake |
| `code` / `code-dev` | VSCodium with this project's extensions baked in |

## Running tests

Three ways, all equivalent:

- **Inline.** rust-analyzer puts `▶ Run Test | Debug` above every `#[test]` and
  every `mod tests`. Click it.
- **The Testing sidebar** (the beaker icon). A tree of every test, with filter,
  re-run, and re-run-failed. Enabled here via `rust-analyzer.testExplorer`.
- **The terminal.**

  ```sh
  cargo test --workspace --all-features          # everything, 79 tests
  cargo test -p argenv --all-features --test scenario -- --nocapture
  cargo test --workspace --all-features -- --list   # every test name
  ```

`--all-features` is not optional: the `contract` feature gates `tests/schema.rs`,
so without it eight tests silently do not run. The editor is configured to pass
it, so the buttons and the terminal agree.

## Tasks and debugging

`.vscode/tasks.json` and `.vscode/launch.json` are committed, because they are
project configuration rather than machine configuration — they contain no store
paths and no personal settings.

**Tasks** (`Ctrl+Shift+P` → *Run Task*, or `Ctrl+Shift+B` for clippy,
`Ctrl+Shift+P` → *Run Test Task* for the suite) run the **same commands as CI,
verbatim**. There is no second way of building or testing the project: a task and
the equivalent terminal line do the same thing. If one changes, change both —
`.github/workflows/ci.yml` is the other copy.

**Debugging.** For a single test, use the `Debug` lens rust-analyzer puts above
it: the configuration is generated for exactly that test, so there is nothing to
keep in sync. `launch.json` covers what a lens cannot — the CLI with arguments
you choose, and the example. Every entry builds through cargo and asks cargo
where the binary landed, so no path is hardcoded.

Editor settings are **not** committed: the flake generates them on every launch,
which is why `.vscode/settings.json` is ignored. Persistent editor configuration
belongs in `dev/flake.nix`.

## Coverage

```sh
coverage                    # writes target/coverage/lcov.info
```

Then press **Watch** in the Coverage Gutters status bar: covered and uncovered
lines appear in the margin and the scrollbar ruler. For a browsable HTML report:

```sh
cargo llvm-cov --workspace --all-features --html --open
```

## What the editor is configured for

- **rust-analyzer runs `clippy -D warnings`**, the same gate as CI, so lints
  appear while typing rather than on push.
- **All features are enabled for analysis.** The `contract` feature gates the
  schema emitter and its tests; without this, `contract.rs` and
  `tests/schema.rs` would be greyed out and unanalysed.
- **The analyser builds into its own target directory**, so it does not fight
  the terminal for the cargo build lock.
- **`schema/` and `api/v1/` open read-only.** They are emitted by
  `argenv schema`; a test fails if a hand edit makes them disagree with the
  model. Regenerate instead:

  ```sh
  cargo run -p argenv-cli -- schema -o schema/argenv-contract.v1.schema.json
  cp schema/argenv-contract.v1.schema.json api/v1/contract.schema.json
  ```

## Notes

- `dev/flake.lock` is committed: everyone gets the same toolchain and the same
  editor build.
- `.ide/` holds the editor's per-project state and is ignored by git. The
  settings file is regenerated on every launch, so persistent configuration
  belongs in `dev/flake.nix`, not in the editor's own settings UI.
- A flake only sees **git-tracked** files. `dev/flake.nix` must be `git add`ed or
  Nix cannot find it.
- If an extension attribute fails to resolve (open-vsx occasionally lags), swap
  that one line to `exts.vscode-marketplace.<publisher>.<name>`.
