# Contract propagation & governance (design notes)

**Principle:** one canonical definition of the *shape*; projects author only
*data*. The schema shape is never hand-copied or hand-edited per project.

## Who owns what

| Artifact | Owner | Hand-edited? |
|---|---|---|
| The shape: `Meta`/`Record` + domain vocabulary (`Version`, `Type`, `Stability`, typed newtypes) | the standard crate (`proton-env-model`) | **Yes — only here**, by the standard holders |
| The JSON-Schema contract (the "swagger") | generated from `Record` via schemars | **Never** — generated |
| Per-language binding stubs (C++/C/Python structs) | generated from the JSON-Schema | **Never** — generated, `// DO NOT EDIT` |
| A project's var list (the *data*) | each project | Yes — this is the only thing a project authors |

## Canonical → project (Rust-first, then generated bindings)

Rust is canonical because it expresses the constraints richest (const-checked
versions, exhaustive enums, range newtypes, compile-checked references) — richer
than any IDL. Flow:

```
proton-env-model (Rust)  --schemars-->  env-contract.schema.json   (the swagger)
                                              |
                    +-------------------------+--------------------------+
                    |                         |                          |
              Rust projects            C / C++ projects            Python projects
           (depend on crate)      (codegen struct from schema)   (codegen dataclass)
                    |                         |                          |
             author vars only          author vars only            author vars only
```

- **Rust consumers** (gamesteam, umu-if-rust): add `proton-env-model` as a
  dependency. They get the exact types; zero drift; they write only `Var`
  declarations. (See `examples/consumer.rs`.)
- **Non-Rust consumers**: a codegen step turns `env-contract.schema.json` into
  their language's record type (committed with a generated-header + a CI check
  that it matches fresh codegen). They then hand-author only their var *data*,
  and their emitted var JSON is validated against the schema in CI.

## Securing the contract

1. **Versioned, additively evolved.** `CONTRACT_VERSION` is semver. New fields
   are optional-with-empty-default (the established pattern), so a bump is
   backward-compatible and old projects keep validating.
2. **Two-way CI validation.**
   - *Shape integrity:* each generated binding is re-generated in CI and diffed
     (`generate && git diff --exit-code`) — a hand-tampered binding fails the build.
   - *Data conformance:* each project's emitted `vars` JSON is validated against
     `env-contract.schema.json` — data that drifts from the contract fails.
3. **Governance.** Field additions/changes go through the standard's repo (the
   "standard holders") as an RFC/PR. Projects propose upstream; they do not fork
   the shape.
4. **Explicit, loud escape hatch.** If a project genuinely can't reach agreement
   with the standard holders, it may hand-edit its local binding — but that
   immediately surfaces as a **CI schema mismatch flagged as "diverged from
   contract vN"**. Divergence is possible but never silent.

## What a project may freely decide (no permission needed)

- *Which* vars it declares and *how many*.
- Each var's data: type, default, allowed set, group, docs, deprecation, etc.
- Its own parsers and how it reads the vars at runtime.

## What a project may NOT do unilaterally

- Change the *shape* of a record (add/rename/retype a field) — that's a contract
  change, owned by the standard.

## Not in scope here (deferred, by request)

- The extractors (static harvest of `getEnvVar("…")` from project source) — a
  later *coverage* safety-net, not the schema source.
- The per-language codegen ports themselves.
- The empirical validation harness (run-a-flag-and-observe).
