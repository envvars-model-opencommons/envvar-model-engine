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
| `nil` `nixpkgs-fmt` | for editing this flake |
| `code` / `code-dev` | VSCodium with this project's extensions baked in |

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
