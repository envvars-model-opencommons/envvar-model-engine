# argenv

**A declared, typed, machine-readable contract for a program's invocation surface — `argv` and `envp`.**

[![CI](https://github.com/argenv-opencommons/argenv/actions/workflows/ci.yml/badge.svg)](https://github.com/argenv-opencommons/argenv/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/argenv.svg)](https://crates.io/crates/argenv)
[![docs.rs](https://img.shields.io/docsrs/argenv)](https://docs.rs/argenv)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](#license)

```rust
pub const LOG_LEVEL: Input<LogLevel> = Input {
    key:     "log_level",                 // identity, transport-free
    ty:      Type::Enum,
    default: Some(LogLevel::Info),
    allowed: LogLevel::TOKENS,            // one source for parser *and* contract
    env:     Some(Env::new("APP_LOG_LEVEL")),
    arg:     Some(Arg { value_name: "LEVEL", ..Arg::pair("log-level", 'l') }),
    summary: "Log verbosity",
    ..Input::EMPTY
};
```

One declaration is simultaneously the **contract entry**, the **typed accessor**, and a
**compile-checked symbol**. Because there is only one artifact, the published contract
cannot drift from the code that reads the value.

`--log-level` and `APP_LOG_LEVEL` are not two settings. They are one setting with two
doors — so they share a type, a domain, a default, and a description.

And because the surface is written down, the failures that are normally silent become
reportable:

```console
$ APP_LOG_LEVL=warn argenv lint myapp.argenv.json -- --hud fps,bogus --hdr=yes
error:   hud contains unknown token `bogus`; accepted: fps, gpuload, memory
error:   --hdr takes no value
error:   licence_key is required; supply it with --licence-key or APP_KEY
warning: APP_LOG_LEVL is not declared; did you mean APP_LOG_LEVEL?
```

That runs against a **published contract document**, so it works for programs written
in any language — including ones that never adopted this crate.

---

## The problem

A program is handed two things when it starts: an argument vector and an environment.
Both are interfaces. Both are depended upon by scripts, container images, CI pipelines,
launchers and people. And almost no program declares either one in a form anything can
check.

The names, the types, the legal values and the defaults live in a README table someone
updates by hand — or in nothing at all, as a string literal compared against another
string literal. Nobody validates them:

- Misspell a flag and you get an error, or worse, silence. Misspell a variable and it is
  **always** silence: no warning, no exit code, just the wrong behaviour discovered later.
- Pass `--verbosity=maybe` where only `0`/`1` are understood and you get whatever the
  parsing code happens to do with garbage.
- Ask *"which inputs does this program actually accept?"* and there is no answer short of
  reading its source.
- Ask *"why is this value what it is?"* — flag, variable, or default? — and nothing can
  tell you.

Config files, by contrast, mostly *do* have schema culture: editor settings, package
manifests, deployment descriptors all routinely ship JSON Schema. The surfaces that are
universally undeclared are the two the operating system hands you. That is the gap this
project names.

## The approaches, and why this one

**1. Write better documentation.** The status quo. Prose has no types, cannot be
validated, cannot be consumed by tooling, and — decisively — drifts the moment anyone
forgets to update it. Documentation *reports* on a contract; it is never the contract.

**2. Extract declarations from source.** Grep for `getenv` and flag literals. Recovers
names and nothing else: no types, no legal values, no defaults, no meaning. Breaks when a
project reshapes its parsing. A floor, not a contract.

**3. Instrument at runtime.** Record what was asked for. Authoritative for what it
observed, structurally blind to everything a particular run did not touch.

**4. Write a separate schema file.** Real and useful — this is what typed-dotenv tools and
CLI description formats do. But a sidecar is a second artifact, and two artifacts drift:
nothing forces the schema and the `getenv` call to agree.

**5. Declare it in code, as the accessor itself.** ← *this project.* The declaration is
what the program actually reads, so it cannot drift. It carries types, domains, defaults,
stability, deprecation and provenance, it is `const` so violations are compile errors, and
it projects to a language-neutral JSON document other stacks can consume.

The design is a middle ground on purpose:

- **Richer than a text schema, cheaper than a framework.** No macros to learn, no registry
  to boot, no runtime. An input is a `const` struct literal.
- **Adoptable one input at a time.** A key, a type and one binding are all that is
  required. A harvested name is a *valid* entry, so a project can start with a list and let
  contributors fill in meaning through one-line pull requests.
- **Neutral about who owns the truth.** A project that adopts nothing can still have its
  surface described externally, in the same format, merged by key.

## What you get

| Artifact | What it is |
|---|---|
| **`argenv` crate** | The typed model: declare inputs, resolve an invocation, read values typed. |
| **`argenv-contract.v1.schema.json`** | The cross-language contract, derived from the model and enriched. |
| **`argenv` CLI** | `schema` emits it; `check` validates documents; `lint` checks a real invocation; `usage` renders help. |
| **The contract API** | Schema and examples as plain GET endpoints, for any language or CI job. |

### Guarantees

- A misspelled key or a wrongly typed read is a **compile error**.
- Absent and invalid are both `None` — an invalid value can never masquerade as a default.
- **Arity is derived from the type**, never declared: a boolean flag takes no value, anything
  else takes one. The derived value is still written out explicitly in the published
  contract, so no other language has to re-derive it.
- Every resolved value carries **where it came from** — argument, environment, or default.
- Two inputs cannot claim the same key, variable or flag; a collision is caught, not shadowed.
- An input that declares no binding at all — one nothing could ever set — is rejected.
- `since` can never exceed the version being built; `reviewed` can never be in the future.
- A deprecated input must carry migration details, and its replacement is a
  **compile-checked reference**: rename the successor and the reference stops compiling.
- Checking an invocation reports misspelled flags *and* variables with suggestions,
  out-of-domain values, unknown list tokens, missing required inputs and deprecated ones in
  use — while leaving the rest of the environment alone, so `PATH` is never mentioned.

## Install

```toml
[dependencies]
argenv = "0.1"
```

## Use

```rust
use argenv::*;

pub struct Model;
impl Model {
    pub const HDR: Input<bool> = Input {
        key:     "hdr",
        ty:      Type::Bool,
        default: Some(false),
        env:     Some(Env::new("MYAPP_HDR")),
        arg:     Some(Arg { negatable: true, ..Arg::long("hdr") }),   // --hdr / --no-hdr
        summary: "Expose HDR output",
        ..Input::EMPTY
    };
}

let args: Vec<String> = std::env::args().skip(1).collect();
let invocation = Invocation { args: &args, env: &ProcessEnv };

// Resolve once; read typed values as often as you like.
let resolved = invocation.resolve(&model);
let hdr: Option<bool> = Model::HDR.get_from_or_default(&resolved);
assert_eq!(resolved.source("hdr"), Some(Source::Arg));   // why it is what it is

// Report anything the invocation got wrong.
for finding in lint(&model, &invocation) {
    eprintln!("{:?}: {finding}", finding.severity());
}
```

```sh
argenv schema -o schema/argenv-contract.v1.schema.json
argenv check  dist/myapp.argenv.json          # is the document well formed?
argenv lint   dist/myapp.argenv.json -- --hdr # does this invocation satisfy it?
argenv usage  dist/myapp.argenv.json          # the help text it implies
```

See [`crates/argenv/examples/consumer.rs`](crates/argenv/examples/consumer.rs) for a
complete model covering both bindings, custom domain types, negation, repetition and
deprecation.

The test suite doubles as the specification: every rule is one test, named as a sentence.
[`tests/scenario.rs`](crates/argenv/tests/scenario.rs) walks the whole life of a contract —
declared, published, read back by an unrelated consumer, and used to catch a broken launch —
and is the fastest way to see the shape of the project. `cargo test` runs all of it,
including whether the schema committed here is still the one the model produces.

## Adopting it in a program that has never declared anything

1. **Harvest the names.** A key, a type and one binding is a valid record. You now have a
   list where there was nothing.
2. **Route reads through the declarations.** Each converted site turns a stringly lookup
   into a compile-checked, typed read with known provenance.
3. **Enrich by pull request.** `summary`, `example`, `allowed`, `reviewed` are one-line
   additions, reviewable by anyone who knows what an input does.
4. **Emit and publish.** Wire `schema` and `check` into CI so the contract is verified on
   every commit and ships as a build artifact.

No step requires the previous one to be complete.

## Project layout

```
crates/argenv/       the model (published to crates.io)
crates/argenv-cli/   schema emission, document validation, invocation checking
schema/              the generated JSON Schema contract
api/                 static GET endpoints (schema + examples)
ARCHITECTURE.md      contract propagation and governance
```

## Status

`0.1` — the model, the schema and the tooling are complete and tested.

The contract format is versioned (`contract_version`) and evolves **additively**: new
fields are optional, records accept properties they do not recognise, and readers ignore
them. A consumer pinned to this version keeps working against documents produced by a later
one — enforced by tests, not promised in prose.

Planned, and deliberately not built yet: a `cfg` binding for configuration-file paths,
man-page and shell-completion generation from a contract, and generated model modules for
other languages. All three consume `Record` and need no change to the model.

## License

MIT OR Apache-2.0, at your option.
