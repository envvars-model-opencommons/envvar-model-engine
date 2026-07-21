//! Example consumer. A project declares ONLY its own var *data*, using the
//! standard's types. Each var is a single flat literal that reads like JSON.
//! No schema shape is reproduced here; no `parse:` field (the type owns parsing).
//!
//! Run:   cargo run --example consumer      (prints the contract JSON)
//!        cargo test --example consumer      (checks the invariants)

use proton_env_model::*;
use std::path::PathBuf;

// Compile-checked successor reference for the deprecated var below:
// rename/remove DXVK_HUD and this stops compiling.
fn hud_replacement() -> &'static str { EnvModel::DXVK_HUD.name }

// Declare the project's vars ONCE; the macro generates the consts + records()/problems().
macro_rules! env_model {
    ( $( $id:ident : $t:ty = $body:expr ; )+ ) => {
        pub struct EnvModel;
        impl EnvModel { $( pub const $id: EnvVar<$t> = $body; )+ }
        impl EnvModel {
            /// Every var rendered to the portable contract record.
            pub fn records() -> Vec<Record> { vec![ $( EnvModel::$id.to_record() ),+ ] }
            /// All invariant violations across the model (empty = valid).
            pub fn problems() -> Vec<String> {
                let mut v = Vec::new(); $( v.extend(EnvModel::$id.check()); )+ v
            }
        }
    };
}

env_model! {
    DXVK_HDR: bool = EnvVar {
        name:      "DXVK_HDR",
        ty:        Type::Bool,
        default:   Some(false),
        allowed:   &["0", "1"],
        stability: Stability::Stable,
        since:     Since::At(Version::parse("2.1")),
        group:     "hdr",
        maps_to:   Some(ConfigKeyRef::new("dxgi.enableHDR")),
        example:   "DXVK_HDR=1",
        observe:   Some("bExposeHDRSupport"),
        reviewed:  Some(ReviewDate::parse("2026-07-21")),
        summary:   "Force-expose HDR10 (G2084) output to the application",
        ..EnvVar::EMPTY
    };

    DXVK_LOG_LEVEL: LogLevel = EnvVar {
        name:      "DXVK_LOG_LEVEL",
        ty:        Type::Enum,
        default:   Some(LogLevel::Info),
        allowed:   LogLevel::TOKENS,          // one source for parser AND schema
        stability: Stability::Stable,
        group:     "logging",
        example:   "DXVK_LOG_LEVEL=warn",
        reviewed:  Some(ReviewDate::parse("2026-07-21")),
        summary:   "Log verbosity",
        ..EnvVar::EMPTY
    };

    // partially described: name + type only. Valid entry; empty reviewed/summary
    // make it show up as an open one-line community PR.
    DXVK_LOG_PATH: PathBuf = EnvVar {
        name:  "DXVK_LOG_PATH",
        ty:    Type::Path,
        group: "logging",
        ..EnvVar::EMPTY
    };

    // int-with-sentinel: 0 = uncapped, modeled as FrameCap (not a bare u32)
    DXVK_FRAME_RATE: FrameCap = EnvVar {
        name:      "DXVK_FRAME_RATE",
        ty:        Type::Uint,
        default:   Some(FrameCap::Uncapped),
        stability: Stability::Stable,
        group:     "frame-pacing",
        example:   "DXVK_FRAME_RATE=60",
        reviewed:  Some(ReviewDate::parse("2026-07-21")),
        summary:   "Frame-rate cap (0 = uncapped)",
        ..EnvVar::EMPTY
    };

    DXVK_HUD: String = EnvVar {
        name:      "DXVK_HUD",
        ty:        Type::Flags,
        allowed:   &["fps", "frametimes", "gpuload", "version", "memory", "submissions"],
        stability: Stability::Stable,
        group:     "hud",
        example:   "DXVK_HUD=fps,gpuload",
        reviewed:  Some(ReviewDate::parse("2026-07-21")),
        summary:   "Comma-separated overlay elements",
        ..EnvVar::EMPTY
    };

    // gamesteam-side: float-with-range domain (RenderScale, not a bare f32)
    GS_RENDER_SCALE: RenderScale = EnvVar {
        name:      "GS_RENDER_SCALE",
        ty:        Type::Float,
        default:   Some(RenderScale::new(1.0)),
        stability: Stability::Stable,
        group:     "gamescope",
        example:   "GS_RENDER_SCALE=0.75",
        reviewed:  Some(ReviewDate::parse("2026-07-21")),
        summary:   "Internal render scale for gamescope FSR upscaling",
        ..EnvVar::EMPTY
    };

    // gamesteam-side: three-state domain (Tristate, not a bool)
    GS_GAMEMODE: Tristate = EnvVar {
        name:      "GS_GAMEMODE",
        ty:        Type::Enum,
        default:   Some(Tristate::Auto),
        allowed:   Tristate::TOKENS,
        stability: Stability::Stable,
        group:     "system",
        example:   "GS_GAMEMODE=auto",
        reviewed:  Some(ReviewDate::parse("2026-07-21")),
        summary:   "Wrap in gamemoderun (auto = on when available)",
        ..EnvVar::EMPTY
    };

    // DEPRECATED: compile-checked replaced_by + freeform migration hint (illustrative var)
    DXVK_PROFILE: bool = EnvVar {
        name:      "DXVK_PROFILE",
        ty:        Type::Bool,
        default:   Some(false),
        stability: Stability::Deprecated,
        deprecation: Some(Deprecation {
            since:       Version::parse("2.3"),
            replaced_by: Some(hud_replacement),
            migration:   "Use DXVK_HUD=submissions,gpuload instead",
        }),
        group:    "hud",
        reviewed: Some(ReviewDate::parse("2026-07-21")),
        summary:  "(deprecated) profiling overlay",
        ..EnvVar::EMPTY
    };
}

fn main() {
    // reads: field access is the typed, name-checked, absence-explicit read
    let hdr: bool = EnvModel::DXVK_HDR.get_or(false);
    let level: LogLevel = EnvModel::DXVK_LOG_LEVEL.get_or(LogLevel::Info);
    let cap: FrameCap = EnvModel::DXVK_FRAME_RATE.get_or(FrameCap::Uncapped);
    let scale: Option<RenderScale> = EnvModel::GS_RENDER_SCALE.get_or_default();
    let gamemode: Tristate = EnvModel::GS_GAMEMODE.get_or(Tristate::Auto);
    if EnvModel::DXVK_LOG_PATH.is_set() { /* redirect logs ... */ }

    // Would NOT compile — the point:
    // let _ = EnvModel::DXVK_HDF.get();          // typo -> no such const
    // let n: u32 = EnvModel::DXVK_HDR.get_or(0);  // wrong type -> EnvVar<bool> != u32

    let _ = (hdr, level, cap, scale, gamemode);

    let doc = document("dxvk@0ff9cd3", EnvModel::records());
    println!("{}", serde_json::to_string_pretty(&doc).unwrap());
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invariants_hold() {
        let problems = EnvModel::problems();
        assert!(problems.is_empty(), "contract violations:\n{:#?}", problems);
    }
}
