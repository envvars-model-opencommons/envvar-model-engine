//! The two doors a value can arrive through: [`Arg`] and [`Env`].
//!
//! An input's *identity* (its `key`) is separate from how it is *addressed*.
//! That separation is the whole reason argv and envp share one model instead of
//! needing two: everything except addressing is common to both.

/// The environment binding: the variable name a value may arrive under.
#[derive(Clone, Copy, Debug)]
pub struct Env {
    /// The variable name, e.g. `"APP_LOG_LEVEL"`.
    pub name: &'static str,
    /// Other names still honoured, so a legacy name is recognised rather than
    /// reported as a mistake.
    pub aliases: &'static [&'static str],
}

impl Env {
    /// A binding with no aliases: `Env { name: "APP_X", ..Env::EMPTY }`.
    pub const EMPTY: Env = Env {
        name: "",
        aliases: &[],
    };

    /// Construct a plain binding.
    pub const fn new(name: &'static str) -> Env {
        Env { name, aliases: &[] }
    }
}

/// The argument-vector binding: the flag a value may arrive under.
///
/// How many values the flag takes is **not** declared here — it follows from the
/// input's [`crate::Type`]. A boolean flag takes none and means *true* by its
/// presence; anything else takes exactly one. Deriving it removes a field that
/// could contradict the type, and the derived arity is still written out
/// explicitly in the published contract so no other language has to re-derive it.
#[derive(Clone, Copy, Debug)]
pub struct Arg {
    /// The long form without dashes, e.g. `"log-level"` for `--log-level`.
    pub long: Option<&'static str>,
    /// The short form without its dash, e.g. `'l'` for `-l`.
    pub short: Option<char>,
    /// Whether `--no-<long>` is accepted to force the value off. Booleans only.
    pub negatable: bool,
    /// Whether the flag may be repeated, accumulating values. Lists only.
    pub repeatable: bool,
    /// The placeholder shown in help for the value, e.g. `"LEVEL"` in
    /// `--log-level <LEVEL>`. Ignored for booleans, which take no value.
    pub value_name: &'static str,
}

impl Arg {
    /// A binding with nothing set: `Arg { long: Some("x"), ..Arg::EMPTY }`.
    pub const EMPTY: Arg = Arg {
        long: None,
        short: None,
        negatable: false,
        repeatable: false,
        value_name: "",
    };

    /// A long flag, e.g. `Arg::long("log-level")` for `--log-level`.
    pub const fn long(long: &'static str) -> Arg {
        Arg {
            long: Some(long),
            ..Arg::EMPTY
        }
    }

    /// A short flag, e.g. `Arg::short('l')` for `-l`.
    pub const fn short(short: char) -> Arg {
        Arg {
            short: Some(short),
            ..Arg::EMPTY
        }
    }

    /// Both forms, e.g. `Arg::pair("log-level", 'l')`.
    pub const fn pair(long: &'static str, short: char) -> Arg {
        Arg {
            long: Some(long),
            short: Some(short),
            ..Arg::EMPTY
        }
    }
}
