//! External tool discovery and process execution.
use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};
pub fn tool(name: &str) -> PathBuf {
    if let Some(p) = std::env::var_os(format!(
        "CASSETTE_{}",
        name.replace('-', "_").to_uppercase()
    )) {
        return p.into();
    }
    let exe = format!("{}{}", name, std::env::consts::EXE_SUFFIX);
    if let Ok(current) = std::env::current_exe()
        && let Some(parent) = current.parent()
    {
        for candidate in [parent.join("tools").join(&exe), parent.join(&exe)] {
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    #[cfg(target_os = "macos")]
    for prefix in ["/opt/homebrew/bin", "/usr/local/bin"] {
        let candidate = Path::new(prefix).join(&exe);
        if candidate.is_file() {
            return candidate;
        }
    }
    let local = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tools")
        .join(&exe);
    if local.is_file() { local } else { exe.into() }
}
pub fn command(name: &str) -> Command {
    let c = Command::new(tool(name));
    #[cfg(windows)]
    let c = {
        let mut c = c;
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x08000000);
        c
    };
    c
}
pub fn available(name: &str) -> bool {
    let path = tool(name);
    path.is_file()
        || std::env::var_os("PATH")
            .is_some_and(|value| std::env::split_paths(&value).any(|dir| dir.join(&path).is_file()))
}
pub fn run(c: &mut Command) -> Result<Output, String> {
    let o = c.output().map_err(|e| {
        format!("外部ツールを起動できません: {e}。セットアップ手順を確認してください")
    })?;
    if !o.status.success() {
        let msg = String::from_utf8_lossy(&o.stderr);
        return Err(format!(
            "音声処理に失敗しました: {}",
            msg.chars()
                .rev()
                .take(1500)
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>()
        ));
    }
    Ok(o)
}
