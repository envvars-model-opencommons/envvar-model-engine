# Changelog

All notable changes are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the crate follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — unreleased

First release. The model covers both surfaces a program is handed at startup.

### The model
- `Input<T>` — one input a program accepts: the contract entry, the typed accessor and a
  compile-checked symbol in one `const`. Identity (`key`) is separate from addressing, so
  a flag and a variable are one setting with two doors.
- `Arg` and `Env` — the two bindings. Long and short forms, negation, repetition and
  aliases. **Arity is derived from the type**, never declared, and written out explicitly
  in the published contract so no consumer has to re-derive it.
- Vocabulary: `Version`, `ReviewDate`, `Type`, `Stability`, `Since`, `ConfigKeyRef`,
  `Deprecation`.
- `FromRaw` — one parsing trait for both surfaces, with implementations for the primitives,
  `PathBuf`, `Tristate` and `LogLevel`.

### Resolving and checking
- `Invocation` and `Resolution` — resolve argv and envp together under a stated
  precedence (`arg` > `env` > `default`), with `Source` on every value so *"why is this
  value what it is?"* has an answer.
- Argument parsing: `--flag`, `--no-flag`, `--opt value`, `--opt=value`, `-f`, `-o value`,
  `-ovalue`, bundled short booleans, and `--` as a terminator. Unrecognised input becomes a
  finding rather than being dropped.
- `lint()` — misspelled flags and variables with suggestions, out-of-domain values,
  unknown list tokens, missing or unexpected flag values, missing required inputs, and
  deprecated inputs in use. Unrelated environment variables are never mentioned.
- `EnvSource` — reads target the process, an environment being assembled for a child, or
  any captured snapshot.

### The contract
- `Record`, `EnvBinding`, `ArgBinding`, `document()`, `check_unique()`.
- `Record::usage()` renders the help line a declaration implies, so documentation has one
  source.
- `contract::json_schema()` — JSON Schema draft 2020-12, derived from the wire types and
  enriched with descriptions, closed sets and formats. Enrichment fails the build if it
  names a field that no longer exists.
- CLI: `schema`, `check`, `lint`, `usage`.

### Compatibility
- Records accept unknown properties in the schema and on deserialisation: the format grows
  by adding optional fields, and a consumer pinned to this version must keep reading newer
  documents.
- `Version` accepts and ignores pre-release and build suffixes, so a project that tags
  `0.2.0-rc1` still builds.
- Three separate name rules, each matching its surface: `snake_case` keys,
  `[A-Za-z_][A-Za-z0-9_]*` variables, kebab-case long flags.
- `reviewed` allows one day of slack against the UTC clock.
