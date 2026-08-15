use std::{fs, path::Path, process::Command};

fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn normalize(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("taskseal", "<entrypoint>").replace("tseal", "<entrypoint>")
}

#[test]
fn locked_local_source_install_owns_only_both_cli_names() {
    assert!(root().join("packaging/cargo/README.md").is_file(), "bounded Cargo install policy is missing");
    let scratch = std::env::temp_dir().join(format!("p07-cargo-install-{}", std::process::id()));
    let _ = fs::remove_dir_all(&scratch);
    fs::create_dir_all(&scratch).unwrap();

    let output = Command::new("cargo")
        .current_dir(root())
        .env("CARGO_NET_OFFLINE", "true")
        .env("CARGO_TARGET_DIR", root().join("target"))
        .args(["install", "--path", ".", "--root", scratch.to_str().unwrap(), "--locked", "--offline", "--force"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let bin = scratch.join("bin");
    let mut names: Vec<_> = fs::read_dir(&bin).unwrap().map(|entry| entry.unwrap().file_name()).collect();
    names.sort();
    assert_eq!(names, ["taskseal", "tseal"]);

    let taskseal = Command::new(bin.join("taskseal")).arg("status").output().unwrap();
    let tseal = Command::new(bin.join("tseal")).arg("status").output().unwrap();
    assert_eq!((taskseal.status.code(), normalize(&taskseal.stdout), normalize(&taskseal.stderr)), (tseal.status.code(), normalize(&tseal.stdout), normalize(&tseal.stderr)));

    let uninstall = Command::new("cargo")
        .current_dir(root())
        .env("CARGO_NET_OFFLINE", "true")
        .args(["uninstall", "--root", scratch.to_str().unwrap(), "taskseal"])
        .output()
        .unwrap();
    assert!(uninstall.status.success(), "{}", String::from_utf8_lossy(&uninstall.stderr));
    assert!(!bin.join("taskseal").exists() && !bin.join("tseal").exists());

    fs::remove_dir_all(&scratch).unwrap();
}
