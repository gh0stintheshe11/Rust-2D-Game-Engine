use std::fs;
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use strip_ansi_escapes::strip;

use rust_2d_game_engine::logger::LOGGER;

/// The engine's license and attribution notice. Apache-2.0 requires both to
/// accompany every distribution of the engine, including games built with
/// it, so builds put them next to the game.
const ENGINE_LICENSE: &str = include_str!("../../../LICENSE");
const ENGINE_NOTICE: &str = include_str!("../../../NOTICE");

/// Build the game with `cargo build --release` and copy its project files
/// next to the executable, so `target/release/` can be run or shipped as is.
pub fn build_project(project_path: &Path) -> Result<(), String> {
    let target_dir = project_path.join("target");
    // The target directory is pinned to the project so the executable lands
    // where the project files are copied, even if the project is a member of
    // a cargo workspace (like the demo) or CARGO_TARGET_DIR is set.
    let mut child = spawn_cargo(
        &[
            "build".as_ref(),
            "--release".as_ref(),
            "--color=always".as_ref(),
            "--target-dir".as_ref(),
            target_dir.as_os_str(),
        ],
        project_path,
    )?;

    let stdout = child.stdout.take().map(|out| forward_lines(out, false));
    let stderr = child.stderr.take().map(|err| forward_lines(err, true));
    let status = child
        .wait()
        .map_err(|e| format!("Failed to wait on build process: {}", e))?;
    for reader in [stdout, stderr].into_iter().flatten() {
        // A reader thread that panicked only loses log lines
        let _ = reader.join();
    }
    if !status.success() {
        return Err("Project build failed".into());
    }

    // Copy the project files next to the executable
    let release_dir = target_dir.join("release");
    for folder in ["assets", "scenes"] {
        let source = project_path.join(folder);
        if source.exists() {
            copy_directory_contents(&source, &release_dir.join(folder))
                .map_err(|e| format!("Failed to copy {}: {}", folder, e))?;
        }
    }
    write_engine_licenses(&release_dir)
        .map_err(|e| format!("Failed to write the engine's license files: {}", e))
}

/// Put the engine's LICENSE and NOTICE in `<dir>/licenses/rust-2d-game-engine/`.
fn write_engine_licenses(dir: &Path) -> io::Result<()> {
    let licenses = dir.join("licenses").join("rust-2d-game-engine");
    fs::create_dir_all(&licenses)?;
    fs::write(licenses.join("LICENSE"), ENGINE_LICENSE)?;
    fs::write(licenses.join("NOTICE"), ENGINE_NOTICE)
}

/// Start `cargo` with `args` in `dir`, with piped output. Editors launched
/// from a desktop environment (e.g. Finder on macOS) often don't have
/// `~/.cargo/bin` on their PATH, so that location is tried too.
fn spawn_cargo(args: &[&std::ffi::OsStr], dir: &Path) -> Result<Child, String> {
    let spawn = |program: &Path| {
        Command::new(program)
            .args(args)
            .current_dir(dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
    };
    match spawn(Path::new("cargo")) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let fallback = cargo_home().map(|home| home.join("bin").join("cargo"));
            match fallback {
                Some(cargo) => spawn(&cargo).map_err(|e| {
                    format!(
                        "Can't run cargo (not on PATH, and {}: {}). Install Rust from https://rustup.rs",
                        cargo.display(),
                        e
                    )
                }),
                None => Err(format!("Can't run cargo: {}", e)),
            }
        }
        other => other.map_err(|e| format!("Can't run cargo: {}", e)),
    }
}

fn cargo_home() -> Option<PathBuf> {
    std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
                .map(|home| PathBuf::from(home).join(".cargo"))
        })
}

/// Echo a child process's output to the terminal (in color) and to the
/// editor console (plain text), line by line. Lines that aren't valid UTF-8
/// are converted lossily instead of ending the stream.
fn forward_lines(
    stream: impl Read + Send + 'static,
    is_stderr: bool,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        for line in BufReader::new(stream).split(b'\n').map_while(Result::ok) {
            let line = String::from_utf8_lossy(&line);
            let line = line.trim_end_matches('\r');
            if is_stderr {
                eprintln!("{}", line);
            } else {
                println!("{}", line);
            }
            LOGGER.debug(String::from_utf8_lossy(&strip(line.as_bytes())));
        }
    })
}

/// Recursively copy directory contents, preserving the structure.
fn copy_directory_contents(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let dst_path = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_directory_contents(&entry.path(), &dst_path)?;
        } else {
            fs::copy(entry.path(), dst_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_licenses_are_written() {
        let dir = std::env::temp_dir().join(format!("rust2d_licenses_{}", uuid::Uuid::new_v4()));
        write_engine_licenses(&dir).unwrap();
        let notice = fs::read_to_string(dir.join("licenses/rust-2d-game-engine/NOTICE")).unwrap();
        assert!(notice.contains("Rust 2D Game Engine"));
        let license = fs::read_to_string(dir.join("licenses/rust-2d-game-engine/LICENSE")).unwrap();
        assert!(license.contains("Apache License"));
    }
}
