use std::process::Command;

fn git_output(mathcat_dir: &std::path::Path, args: &[&str]) -> String {
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

fn git_path(mathcat_dir: &std::path::Path, git_path: &str) -> std::path::PathBuf {
    let path = std::path::PathBuf::from(git_path);
    if path.is_absolute() {
        path
    } else {
        mathcat_dir.join(path)
    }
}

fn main() {
    let manifest_dir = std::path::PathBuf::from(
        std::env::var_os("CARGO_MANIFEST_DIR").expect("Cargo did not set CARGO_MANIFEST_DIR"),
    );
    let mathcat_dir = manifest_dir.join("../MathCAT");

    let timestamp = git_output(&mathcat_dir, &["log", "-1", "--format=%cI"]);
    let (date, time) = timestamp
        .split_once('T')
        .expect("MathCAT commit timestamp is not in RFC 3339 format");
    let time = time
        .get(..5)
        .expect("MathCAT commit timestamp does not contain hours and minutes");
    println!("cargo:rustc-env=MATHCAT_BUILD_DATE={date} {time}");

    let head_path = git_output(&mathcat_dir, &["rev-parse", "--git-path", "HEAD"]);
    println!(
        "cargo:rerun-if-changed={}",
        git_path(&mathcat_dir, &head_path).display()
    );
    let branch = Command::new("git")
        .arg("-C")
        .arg(&mathcat_dir)
        .args(["symbolic-ref", "--quiet", "HEAD"])
        .output()
        .expect("failed to read the MathCAT branch reference");
    if branch.status.success() {
        let branch_ref = String::from_utf8_lossy(&branch.stdout).trim().to_owned();
        let branch_path = git_output(&mathcat_dir, &["rev-parse", "--git-path", &branch_ref]);
        println!(
            "cargo:rerun-if-changed={}",
            git_path(&mathcat_dir, &branch_path).display()
        );
    }

    println!("cargo:rerun-if-changed=../MathCAT/Cargo.toml");
}
