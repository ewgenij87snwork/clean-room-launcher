use std::{env, fs};

fn main() {
    let capture = env::var("TASKSEAL_CAPTURE_PATH").expect("capture path is required");
    let args = env::args().skip(1).collect::<Vec<_>>().join("\0");
    let marker = env::var("TASKSEAL_INHERITED_MARKER").unwrap_or_default();
    fs::write(capture, format!("{args}\0{marker}")).expect("capture must be writable");
    match env::args().nth(1).as_deref() {
        Some("--exit-42") => std::process::exit(42),
        Some("--abort") => std::process::abort(),
        _ => {}
    }
}
