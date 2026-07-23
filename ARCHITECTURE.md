# Contract propagation & governance

**Principle:** one canonical definition of the *shape*; projects author only *data*.
The shape is never hand-copied or hand-edited per project.

## Who owns what

| Artifact | Owner | Hand-edited? |
|---|---|---|
| The shape: `Input`/`Record` + vocabulary (`Version`, `ReviewDate`, `Type`, `Stability`, `Since`, `ConfigKeyRef`, `Deprecation`) | the `argenv` crate | **Yes — only here**, by the standard holders |
| The checker: `lint`, `EnvSource` | the `argenv` crate | Yes — only here |
| `argenv-contract.v1.schema.json` | derived from `Record` by the CLI | **Never** — generated |
| Per-language binding stubs (C/C++/Python) | generated from the JSON Schema | **Never** — generated, `DO NOT EDIT` |
| A project's variable list (the *data*) | each project | Yes — the only thing a project authors |

## Canonical → project

Rust is canonical because it expresses the constraints richest: `const`-validated
versions and dates, exhaustive enums, range newtypes, and compile-checked
references. No IDL can enforce those at authoring time.

```
argenv (Rust)  --derive+enrich-->  argenv-contract.v1.schema.json
                                                      |
              +---------------------------------------+---------------------+
              |                        |                                    |
        Rust projects           C / C++ projects                    Python projects
     (depend on the crate)   (codegen record from schema)     (codegen dataclass)
              |                        |                                    |
        author data only         author data only                   author data only
```

- **Rust consumers** add the crate and get the exact types; zero drift.
- **Non-Rust consumers** generate their record type from the schema, commit it with
  a generated header, and validate their emitted documents against the schema in CI.

## How the schema stays honest

The schema is **derived** from the `Record` struct by `schemars`, then *enriched*
with descriptions, closed sets (`Type::ALL`, `Stability::ALL`), and format patterns.
The enrichment step looks every property up by name and **panics if it is missing**,
so renaming a field without updating its documentation fails the build instead of
silently dropping it. A test additionally asserts that every field carries a
description and that the closed sets match the Rust enums.

## How the format evolves without breaking consumers

Additive change is the only routine change, and it is guaranteed on three levels
so a consumer pinned to `v1` keeps working against a document produced later:

- the JSON Schema permits unknown properties on a record;
- `Record` ignores unknown fields when deserialising;
- `CONTRACT_VERSION` gates only genuinely breaking changes, which get a new path.

Authoring mistakes are still caught: `argenv check` reports a field it does
not recognise, without declaring the document invalid. Strictness belongs in the
tool, not in the format — a format that refuses the unfamiliar cannot grow.

## Securing the contract

1. **Versioned, additively evolved.** `CONTRACT_VERSION` gates breaking changes.
   New fields are optional, so a minor release keeps old documents valid.
2. **Two-way CI validation.**
   - *Shape integrity:* the schema is regenerated and diffed (`git diff --exit-code`)
     — a hand-tampered schema fails the build.
   - *Data conformance:* every emitted document is validated with
     `argenv check` and against the JSON Schema.
3. **Governance.** Field additions or changes go through this repository as an
   RFC/pull request. Projects propose upstream; they do not fork the shape.
4. **Explicit, loud escape hatch.** A project that cannot reach agreement may
   hand-edit its local binding — which immediately surfaces as a schema mismatch
   flagged as *diverged from contract vN*. Divergence is possible, never silent.

## What a project decides freely

Which variables it declares, and every field of their data: type, default, allowed
tokens, group, prose, deprecation, review dates, and how it reads them at runtime.

## What a project may not do unilaterally

Change the *shape* of a record — add, rename, or retype a field. That is a contract
change, owned by the standard.

## Extending the model

Two seams are designed to absorb growth without touching the core.

**A third binding.** `Arg` and `Env` are independent structs on `Input`, and the wire type
mirrors them as independent optional objects. A configuration binding is the same shape:

```rust
pub struct Cfg { pub path: &'static str }   // "logging.level"
```

Adding one means a new field on `Input` and `Record`, a new object in the schema, and a
new arm in resolution — all additive, none of it breaking a pinned consumer. What must
*not* move into the model is locating and parsing configuration files: which file, which
format, which merge order. The contract says an input may arrive at `logging.level`; a
TOML or YAML adapter resolves it. Keeping that split is why `argenv` covers the two
surfaces the operating system defines, and treats everything else as an adapter.

**Generators.** Man pages, shell completions and language modules are pure functions of
`Record`, which already carries everything they need: `usage()` for the signature,
`summary` and `group` for the text, `allowed` for completion candidates, `arity` for
completion behaviour, `env_names()` for the environment section. They belong in separate
crates that depend on `argenv` and consume a document — no model change, no coupling, and
they work equally on contracts published by programs written in other languages.

## Deliberately out of scope (for now)

- **Static extractors** that harvest `getenv("…")` names from third-party source.
  Useful as a *coverage* safety net — "which variables are not yet declared?" — not
  as a source of truth.
- **Per-language codegen ports** (C/C++/Python record generators).
- **The empirical validation harness**: set a variable, run the program, and confirm
  the field named by `observe` actually moved. The `observe` field exists to make
  this possible later; a variable that changes nothing observable is either
  misdocumented or fictional.
- **Merging registries.** Combining documents from several projects needs a
  precedence policy — which source wins per field, and how confidence is ranked.
  `Record::source` and `Deserialize` make the merge possible; the policy is a
  governance decision and is deliberately not baked in yet.
