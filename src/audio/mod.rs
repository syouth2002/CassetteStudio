//! Audio facade: callers do not depend on individual processing modules.
mod analysis;
mod import;
mod metadata;
mod processing;
mod render;
mod tools;

pub use analysis::measure_track;
pub use import::{analyze, import_url};
pub use metadata::repair_legacy_tag;
pub use processing::{gain, preview_name};
pub use render::{preview, render};
pub use tools::available;

// Measurements and playback must use the same stereo conversion.
const STEREO: &str = "aresample=44100,aformat=sample_fmts=fltp:channel_layouts=stereo";

#[cfg(test)]
mod tests;
