use crate::{config::Config, BoxResult};
use std::{
    ffi::OsString,
    process::{Command, ExitStatus},
};

pub fn doc(
    config: &Config,
    args: &mut pico_args::Arguments,
    tool_args: Vec<OsString>,
) -> BoxResult<Option<ExitStatus>> {
    let help = r#"
xtask-doc

USAGE:
xtask doc

FLAGS:
-h, --help          Prints help information
-- '...'            Extra arguments to pass to the cargo command
"#
    .trim();

    if crate::handler::help(args, help)? {
        return Ok(None);
    }

    crate::handler::unused(args)?;

    let toolchain = crate::config::rust::toolchain::nightly(config);

    crate::validation::validate_rust_toolchain(&toolchain)?;

    let env_vars = crate::validation::validate_tool(config, "cargo-doc")?;

    let mut cmd = Command::new("cargo");
    cmd.current_dir(crate::cargo::project_root()?);
    cmd.args([&format!("+{toolchain}"), "doc"]);
    cmd.args(tool_args);
    for (key, value) in env_vars {
        cmd.env(key, value);
    }
    let status = cmd.status()?;

    Ok(Some(status))
}