use serde::Deserialize;
use std::collections::HashMap;

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize)]
pub struct Xtask {
    pub clang: XtaskClang,
    pub rust: XtaskRust,
}

#[allow(clippy::module_name_repetitions)]
#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize)]
pub struct XtaskClang {
    pub matchers: HashMap<String, String>,
    pub platform: XtaskPlatform,
    pub suffix: String,
    pub version: String,
}

#[allow(clippy::module_name_repetitions)]
#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize)]
pub struct XtaskPlatform {
    pub macos: XtaskPlatformMacos,
}

#[allow(clippy::module_name_repetitions)]
#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XtaskPlatformMacos {
    pub search_paths: Vec<XtaskPlatformMacosSearchPath>,
}

#[allow(clippy::module_name_repetitions)]
#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum XtaskPlatformMacosSearchPath {
    Homebrew,
}

#[allow(clippy::module_name_repetitions)]
#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize)]
pub struct XtaskRust {
    pub components: HashMap<String, XtaskRustComponent>,
    pub toolchain: XtaskRustToolchain,
}

#[allow(clippy::module_name_repetitions)]
#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize)]
pub struct XtaskRustComponent {
    pub toolchain: String,
}

#[allow(clippy::module_name_repetitions)]
#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize)]
pub struct XtaskRustToolchain {
    pub nightly: String,
}