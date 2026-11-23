use std::path::{Path, PathBuf};
use std::process::Command;

fn git_output(mathcat_dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(mathcat_dir)
        .args(args)
        .output()
        .expect("failed to run git while reading MathCAT build metadata");

    if !output.status.success() {
        panic!(
            "failed to read MathCAT build metadata: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    String::from_utf8(output.stdout)
        .expect("git returned non-UTF-8 MathCAT build metadata")
        .trim()
        .to_owned()
}

fn git_path(mathcat_dir: &Path, git_path: &str) -> PathBuf {
    let path = PathBuf::from(git_path);
    if path.is_absolute() {
        path
    } else {
        mathcat_dir.join(path)
    }
}

fn main() {
    let manifest_dir = PathBuf::from(
        std::env::var_os("CARGO_MANIFEST_DIR").expect("Cargo did not set CARGO_MANIFEST_DIR"),
    );
    // Canonicalize so Cargo fingerprints a stable path (no `..`), which matters on Windows.
    let mathcat_dir = std::fs::canonicalize(manifest_dir.join("../MathCAT"))
        .expect("failed to locate sibling MathCAT checkout at ../MathCAT");

    let timestamp = git_output(&mathcat_dir, &["log", "-1", "--format=%cI"]);
    let (date, time) = timestamp
        .split_once('T')
        .expect("MathCAT commit timestamp is not in RFC 3339 format");
    let time = time
        .get(..5)
        .expect("MathCAT commit timestamp does not contain hours and minutes");
    println!("cargo:rustc-env=MATHCAT_BUILD_DATE={date} {time}");

    let git_dir = git_path(
        &mathcat_dir,
        &git_output(&mathcat_dir, &["rev-parse", "--absolute-git-dir"]),
    );
    // HEAD changes on checkout; branch tips under refs/heads change on commit/pull.
    // Watching only the branch that was current at the last build.rs run misses
    // switches to another branch (HEAD stays a symref; the old tip file is unchanged).
    println!("cargo:rerun-if-changed={}", git_dir.join("HEAD").display());
    println!(
        "cargo:rerun-if-changed={}",
        git_dir.join("packed-refs").display()
    );
    let refs_heads = git_dir.join("refs").join("heads");
    if refs_heads.is_dir() {
        println!("cargo:rerun-if-changed={}", refs_heads.display());
    }

    println!(
        "cargo:rerun-if-changed={}",
        mathcat_dir.join("Cargo.toml").display()
    );
}
