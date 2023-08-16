use crate::{config::Config, BoxError, BoxResult};
use regex::Regex;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    ffi::OsString,
    process::Command,
};

#[derive(Debug, Default)]
pub struct Validation {
    pub tools: HashMap<String, String>,
    pub env_vars: BTreeMap<OsString, OsString>,
}

impl Validation {
    pub fn combine(&mut self, other: Validation) {
        self.tools.extend(other.tools);
        self.env_vars.extend(other.env_vars);
    }
}

/// # Errors
///
/// Will return `Err` if tool validation fails (missing tools, incorrect versions, etc).
pub fn validate_tool(config: &Config, tool: &str) -> BoxResult<Validation> {
    let mut validation = Validation::default();
    #[allow(clippy::match_same_arms)]
    match tool {
        "clang" => {
            validation.combine(validate_clang_tool(config, tool)?);
        },
        "clang++" => {
            validation.combine(validate_clang_tool(config, tool)?);
        },
        "clangd" => {
            validation.combine(validate_clang_tool(config, tool)?);
        },
        "clang-format" => {
            validation.combine(validate_clang_tool(config, tool)?);
            validate_python3()?;
            validate_xtask_bin(config, "run-clang-format.py", true)?;
        },
        "clang-tidy" => {
            validation.combine(validate_clang_tool(config, tool)?);
            validation.combine(validate_clang_tool(config, "run-clang-tidy")?);
        },
        "cargo-clippy" => {
            validate_cargo_component(config, tool)?;
        },
        "cargo-fmt" => {
            validate_cargo_component(config, tool)?;
        },
        "cargo-miri" => {
            validate_cargo_component(config, tool)?;
        },
        "cargo-tarpaulin" => {
            validate_cargo_tool(tool)?;
        },
        "cargo-udeps" => {
            validate_cargo_tool(tool)?;
        },
        "cargo-valgrind" => {
            validate_cargo_tool(tool)?;
        },
        "cmake" => {
            validate_other_tool(tool, &validation.env_vars)?;
        },
        "ninja" => {
            validate_other_tool(tool, &validation.env_vars)?;
        },
        _ => {
            return Err(format!("unrecognized tool: `{tool}`").into());
        },
    }
    Ok(validation)
}

fn validate_cargo_component(config: &Config, tool: &str) -> BoxResult<()> {
    let tool = tool.strip_prefix("cargo-").unwrap_or(tool);
    let component_name = if tool == "doc" { "rustdoc" } else { tool };
    if let Some(value) = config.xtask.rust.components.get(component_name) {
        let toolchain = if value.toolchain == "nightly" {
            crate::config::rust::toolchain::nightly(config)
        } else {
            crate::config::rust::toolchain::stable(config)
        };
        let mut cmd = Command::new("cargo");
        cmd.args([&format!("+{toolchain}"), tool]);
        cmd.args(["--help"]);
        cmd.stdout(std::process::Stdio::null());
        if !cmd.status()?.success() {
            return Err(format!("`cargo +{toolchain} {tool} --help` failed with non-zero exit code").into());
        }
    } else {
        return Err(format!("unrecognized component: `{tool}`").into());
    }
    Ok(())
}

fn validate_cargo_tool(tool: &str) -> BoxResult<()> {
    let mut cmd = Command::new(tool);
    cmd.args(["--help"]);
    cmd.stdout(std::process::Stdio::null());
    let status = cmd.status().map_err(|err| -> BoxError {
        if err.kind() == std::io::ErrorKind::NotFound {
            format!("could not find `{tool}` in path").into()
        } else {
            err.into()
        }
    })?;
    if !status.success() {
        return Err(format!("`{tool}` failed with non-zero exit code").into());
    }
    Ok(())
}

fn validate_clang_tool(config: &Config, tool: &str) -> BoxResult<Validation> {
    let matcher = config
        .xtask
        .clang
        .matchers
        .get(tool)
        .map(|matcher| regex::Regex::new(matcher))
        .transpose()?;
    let matcher = matcher.as_ref();

    // check `tool` with suffix
    {
        let tool_elaborated = Some(format!("{tool}{}", config.xtask.clang.suffix));
        let env_vars = BTreeMap::default();
        if let Ok(validation) = try_validate_clang_tool(config, matcher, tool, tool_elaborated.as_deref(), env_vars) {
            return Ok(validation);
        }
    }

    // check `tool` with suffix in search paths
    {
        #[allow(unused_mut)]
        let mut paths = if let Some(path) = std::env::var_os("PATH") {
            std::env::split_paths(&path).collect::<BTreeSet<_>>()
        } else {
            BTreeSet::default()
        };
        // on macOS, add the homebrew install location to the PATH for a final test
        #[cfg(target_os = "macos")]
        paths.extend(crate::detection::detect_macos_clang_paths(config)?);
        let path = std::env::join_paths(paths)?;
        let tool_elaborated = Some(format!("{tool}{}", config.xtask.clang.suffix));
        let env_vars = BTreeMap::from_iter([("PATH".into(), path)]);
        if let Ok(validation) = try_validate_clang_tool(config, matcher, tool, tool_elaborated.as_deref(), env_vars) {
            return Ok(validation);
        }
    }

    // check `tool` with no suffix
    {
        let tool_elaborated = None;
        let env_vars = BTreeMap::default();
        if let Ok(validation) = try_validate_clang_tool(config, matcher, tool, tool_elaborated, env_vars) {
            return Ok(validation);
        }
    }

    // check `tool` with no suffix in search paths
    {
        #[allow(unused_mut)]
        let mut paths = if let Some(path) = std::env::var_os("PATH") {
            std::env::split_paths(&path).collect::<BTreeSet<_>>()
        } else {
            BTreeSet::default()
        };
        // on macOS, add the homebrew install location to the PATH for a final test
        #[cfg(target_os = "macos")]
        paths.extend(crate::detection::detect_macos_clang_paths(config)?);
        let path = std::env::join_paths(paths)?;
        let tool_elaborated = None;
        let env_vars = BTreeMap::from_iter([("PATH".into(), path)]);
        if let Ok(validation) = try_validate_clang_tool(config, matcher, tool, tool_elaborated, env_vars) {
            return Ok(validation);
        }
    }
    // }

    Err(format!("could not validate tool: `{tool}`").into())
}

fn try_validate_clang_tool(
    config: &Config,
    matcher: Option<&Regex>,
    tool: &str,
    tool_elaborated: Option<&str>,
    env_vars: BTreeMap<OsString, OsString>,
) -> BoxResult<Validation> {
    let tool_elaborated = tool_elaborated.unwrap_or(tool);
    let mut cmd = Command::new(tool_elaborated);

    // Configure the invocation to test the tool with `--version` or `--help`.
    if matcher.is_some() {
        cmd.arg("--version");
    } else {
        // Some tools don't support `--version` and hence don't have a matcher. Also, they will
        // return non-zero exit on invocation with no arguments, so we invoke with `--help`.
        cmd.arg("--help");
    }

    // Configure the invocation to use the specified environment variables.
    for (key, value) in &env_vars {
        cmd.env(key, value);
    }

    // Invoke the tool and capture its output.
    let output = cmd.output()?;

    // If the invocation was successful, try to validate the output.
    if output.status.success() {
        // Initialize the validation result.
        let mut validation = Validation {
            env_vars,
            ..Default::default()
        };
        // If the tool name was elaborated to something more specific (e.g., with a suffix `-16`),
        // update the tool name mappings in the validation result ...
        if tool_elaborated != tool {
            let tool = String::from(tool);
            let tool_elaborated = String::from(tool_elaborated);
            validation.tools.insert(tool, tool_elaborated);
        }
        // If the tool has an associated matcher, try to validate the output with that ...
        if let Some(matcher) = matcher {
            let haystack = String::from_utf8(output.stdout)?;
            // Initialize the regex from the tool matcher and try to capture and check the version number.
            if let Some(version) = matcher
                .captures(&haystack)
                .and_then(|captures| captures.get(1).map(|m| m.as_str()))
            {
                if version.starts_with(&config.xtask.clang.version) {
                    return Ok(validation);
                }
                let message = format!(
                    "`{tool}` failed validation; expected version compatible with `{version}` but found `{actual}`",
                    version = config.xtask.clang.version,
                    actual = version,
                );
                return Err(message.into());
            }
            let message = format!("`{tool}` failed validation; ensure you are using the official clang toolchain");
            Err(message.into())
        }
        // ... otherwise, assume that the invocation succeeding is enough and just return the validation result.
        else {
            Ok(validation)
        }
    } else {
        let message = format!("`{tool}` failed with non-zero exit code");
        Err(message.into())
    }
}

fn validate_other_tool(tool: &str, env_vars: &BTreeMap<OsString, OsString>) -> BoxResult<Validation> {
    let mut cmd = Command::new(tool);
    match tool {
        "ninja" => cmd.args(["--version"]),
        _ => cmd.args(["--help"]),
    };
    cmd.stderr(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::null());
    for (key, value) in env_vars {
        cmd.env(key, value);
    }
    let status = cmd.status().map_err(|err| -> BoxError {
        if err.kind() == std::io::ErrorKind::NotFound {
            format!("could not find `{tool}` in path").into()
        } else {
            err.into()
        }
    })?;
    if !status.success() {
        return Err(format!("`{tool}` failed with non-zero exit code").into());
    }
    Ok(Validation::default())
}

fn validate_python3() -> BoxResult<()> {
    let mut cmd = Command::new("python3");
    cmd.args(["--help"]);
    cmd.stdout(std::process::Stdio::null());
    let status = cmd.status().map_err(|err| -> BoxError {
        if err.kind() == std::io::ErrorKind::NotFound {
            format!("could not find `python3` in path").into()
        } else {
            err.into()
        }
    })?;
    if !status.success() {
        return Err("`python3` failed with non-zero exit code".into());
    }
    Ok(())
}

/// # Errors
///
/// Will return `Err` under the following circumstances:
/// - The command process for `rustup toolchain list` fails to start
/// - The command invocation fails with non-zero exit status
/// - The command invocation fails to produce valid UTF-8 output
/// - The specified `toolchain` is not found in the output of `rustup toolchain list`
pub fn validate_rust_toolchain(toolchain: &str) -> BoxResult<()> {
    let mut cmd = Command::new("rustup");
    cmd.args(["toolchain", "list"]);
    let output = cmd.output()?;
    if output.status.success() {
        for entry in String::from_utf8(output.stdout)?.lines() {
            if entry.starts_with(toolchain) {
                return Ok(());
            }
        }
    } else {
        return Err("`rustup toolchain list` failed with non-zero exit code".into());
    }
    Err(format!(
        "could not find toolchain `{toolchain}`\nPerhaps you need to install it with `rustup install toolchain {toolchain}`?",
    )
    .into())
}

fn validate_xtask_bin(config: &Config, tool: &str, retry: bool) -> BoxResult<()> {
    let needs_install = match tool {
        "run-clang-format.py" => {
            let tool = config.xtask_bin_dir.join(tool);
            let mut cmd = Command::new("python3");
            cmd.args([tool.as_os_str(), "--help".as_ref()]);
            cmd.stderr(std::process::Stdio::null());
            cmd.stdout(std::process::Stdio::null());
            if cmd.status()?.success() {
                Ok::<_, BoxError>(None)
            } else {
                Ok::<_, BoxError>(Some(
                    "https://raw.githubusercontent.com/Sarcasm/run-clang-format/master/run-clang-format.py",
                ))
            }
        },
        _ => {
            return Err(format!("unrecognized xtask bin `{tool}`").into());
        },
    }?;
    if let Some(url) = needs_install {
        crate::install::fetch_xtask_bin(config, url, tool)?;
        if retry {
            validate_xtask_bin(config, tool, false)?;
        }
    }
    Ok(())
}