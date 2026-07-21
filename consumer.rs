//! Example consumer of the standard.
//!
//! A project (here: some DXVK vars + a few gamesteam-side vars) declares ONLY its
//! own var *data*, using the standard's types. It never reproduces the schema
//! shape — `Meta`, `Var`, `Version`, `LogLevel`, ... all come from the crate.
//!
//! Run:   cargo run --example consumer        (prints the contract JSON)
//!        cargo test --example consumer        (checks the invariants)

use proton_env_model::*;

// -- project-local parsers for the plain types (parsing is a project concern;
//    the rich domain types ship their own parse fns in the standard). --
fn p_bool(s: &str) -> Option<bool> {
    match s.trim() {
        "1" | "true" | "on" => Some(true),
        "0" | "false" | "off" | "" => Some(false),
        _ => None,
    }
}
fn p_path(s: &str) -> Option<std::path::PathBuf> {
    if s.is_empty() { None } else { Some(s.into()) }
}
fn p_string(s: &str) -> Option<String> { Some(s.to_string()) }

// Compile-checked successor reference used by the deprecated var below:
// reads DXVK_HUD's name; rename/remove DXVK_HUD and this stops compiling.
fn hud_replacement() -> &'static str { EnvModel::DXVK_HUD.meta.name }

// ---------------------------------------------------------------------------
// Declare the project's vars ONCE. The macro generates the named consts plus
// `metas()`/`records()`, so the var list has a single source of truth.
// ---------------------------------------------------------------------------
macro_rules! env_model {
    ( $( $id:ident : $t:ty = $body:expr ; )+ ) => {
        pub struct EnvModel;
        impl EnvModel { $( pub const $id: Var<$t> = $body; )+ }
        impl EnvModel {
            /// Every var's authoring metadata (for validation).
            pub fn metas() -> Vec<Meta> { vec![ $( EnvModel::$id.meta ),+ ] }
            /// Every var rendered to the portable contract record (for emission).
            pub fn records() -> Vec<Record> { vec![ $( EnvModel::$id.to_record() ),+ ] }
        }
    };
}

env_model! {
    // fully-described: bool with a config bridge, HDR observable, reviewed
    DXVK_HDR: bool = Var {
        meta: Meta {
            name: "DXVK_HDR", ty: Type::Bool, allowed: &["0", "1"],
            stability: Stability::Stable, since: Since::At(Version::parse("2.1")),
            group: "hdr", maps_to: Some(ConfigKeyRef::new("dxgi.enableHDR")),
            example: "DXVK_HDR=1", observe: Some("bExposeHDRSupport"),
            reviewed: Some(ReviewDate::parse("2026-07-21")),
            summary: "Force-expose HDR10 (G2084) output to the application",
            ..Meta::EMPTY
        },
        default: Some(false), parse: p_bool
    };

    // enum: `allowed` is LogLevel::TOKENS — one source for parser AND schema
    DXVK_LOG_LEVEL: LogLevel = Var {
        meta: Meta {
            name: "DXVK_LOG_LEVEL", ty: Type::Enum, allowed: LogLevel::TOKENS,
            stability: Stability::Stable, group: "logging", example: "DXVK_LOG_LEVEL=warn",
            reviewed: Some(ReviewDate::parse("2026-07-21")),
            summary: "Log verbosity",
            ..Meta::EMPTY
        },
        default: Some(LogLevel::Info), parse: LogLevel::parse
    };

    // partially described: known name + type only. Valid entry; `reviewed`/`summary`
    // empty => shows up as UNVERIFIED/UNDOCUMENTED, an open one-line community PR.
    DXVK_LOG_PATH: std::path::PathBuf = Var {
        meta: Meta { name: "DXVK_LOG_PATH", ty: Type::Path, group: "logging", ..Meta::EMPTY },
        default: None, parse: p_path
    };

    // int-with-sentinel: 0 = uncapped, modeled as FrameCap (not a bare u32)
    DXVK_FRAME_RATE: FrameCap = Var {
        meta: Meta {
            name: "DXVK_FRAME_RATE", ty: Type::Uint, stability: Stability::Stable,
            group: "frame-pacing", example: "DXVK_FRAME_RATE=60",
            reviewed: Some(ReviewDate::parse("2026-07-21")),
            summary: "Frame-rate cap (0 = uncapped)",
            ..Meta::EMPTY
        },
        default: Some(FrameCap::Uncapped), parse: FrameCap::parse
    };

    // flags (token list); successor of the deprecated var below
    DXVK_HUD: String = Var {
        meta: Meta {
            name: "DXVK_HUD", ty: Type::Flags,
            allowed: &["fps", "frametimes", "gpuload", "version", "memory", "submissions"],
            stability: Stability::Stable, group: "hud", example: "DXVK_HUD=fps,gpuload",
            reviewed: Some(ReviewDate::parse("2026-07-21")),
            summary: "Comma-separated overlay elements",
            ..Meta::EMPTY
        },
        default: None, parse: p_string
    };

    // gamesteam-side: float-with-range domain (RenderScale, not a bare f32)
    GS_RENDER_SCALE: RenderScale = Var {
        meta: Meta {
            name: "GS_RENDER_SCALE", ty: Type::Float, stability: Stability::Stable,
            group: "gamescope", example: "GS_RENDER_SCALE=0.75",
            reviewed: Some(ReviewDate::parse("2026-07-21")),
            summary: "Internal render scale for gamescope FSR upscaling",
            ..Meta::EMPTY
        },
        default: Some(RenderScale::new(1.0)), parse: RenderScale::parse
    };

    // gamesteam-side: three-state domain (Tristate, not a bool)
    GS_GAMEMODE: Tristate = Var {
        meta: Meta {
            name: "GS_GAMEMODE", ty: Type::Enum, allowed: Tristate::TOKENS,
            stability: Stability::Stable, group: "system", example: "GS_GAMEMODE=auto",
            reviewed: Some(ReviewDate::parse("2026-07-21")),
            summary: "Wrap in gamemoderun (auto = on when available)",
            ..Meta::EMPTY
        },
        default: Some(Tristate::Auto), parse: Tristate::parse
    };

    // DEPRECATED: compile-checked `replaced_by` pointer + freeform migration hint.
    // (illustrative var, to show the deprecation mechanism)
    DXVK_PROFILE: bool = Var {
        meta: Meta {
            name: "DXVK_PROFILE", ty: Type::Bool, stability: Stability::Deprecated,
            deprecation: Some(Deprecation {
                since: Version::parse("2.3"),
                replaced_by: Some(hud_replacement),
                migration: "Use DXVK_HUD=submissions,gpuload instead",
            }),
            group: "hud", reviewed: Some(ReviewDate::parse("2026-07-21")),
            summary: "(deprecated) profiling overlay",
            ..Meta::EMPTY
        },
        default: Some(false), parse: p_bool
    };
}

fn main() {
    // --- reads: field access is the typed, name-checked, absence-explicit read ---
    let hdr: bool = EnvModel::DXVK_HDR.get_or(false);
    let level: LogLevel = EnvModel::DXVK_LOG_LEVEL.get_or(LogLevel::Info);
    let cap: FrameCap = EnvModel::DXVK_FRAME_RATE.get_or(FrameCap::Uncapped);
    let scale: Option<RenderScale> = EnvModel::GS_RENDER_SCALE.get_or_default();
    let gamemode: Tristate = EnvModel::GS_GAMEMODE.get_or(Tristate::Auto);
    if EnvModel::DXVK_LOG_PATH.is_set() { /* redirect logs ... */ }

    // These would NOT compile — the whole point:
    // let _ = EnvModel::DXVK_HDF.get();            // typo -> no such associated const
    // let n: u32 = EnvModel::DXVK_HDR.get_or(0);    // wrong type -> Var<bool> != u32

    let _ = (hdr, level, cap, scale, gamemode);

    // --- emit this project's slice of the contract as JSON ---
    // (`source` is stamped by the build; hardcoded here for the example)
    let doc = document("dxvk@0ff9cd3", EnvModel::records());
    println!("{}", serde_json::to_string_pretty(&doc).unwrap());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The standard's invariants hold for THIS project's declarations:
    /// no future review dates, no since > current, deprecation consistency, etc.
    #[test]
    fn invariants_hold() {
        let problems = validate(&EnvModel::metas());
        assert!(problems.is_empty(), "contract violations:\n{:#?}", problems);
    }
}
