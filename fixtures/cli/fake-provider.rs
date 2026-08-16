use std::{env, fs, io::Read};

fn main() {
    let capture = env::var("CLROOM_CAPTURE_PATH").expect("capture path is required");
    let args = env::args().skip(1).collect::<Vec<_>>().join("\0");
    let marker = env::var("CLROOM_INHERITED_MARKER").unwrap_or_default();
    fs::write(capture, format!("{args}\0{marker}")).expect("capture must be writable");
    match env::args().nth(1).as_deref() {
        Some("--exit-42") => std::process::exit(42),
        Some("--abort") => std::process::abort(),
        Some("--stdio") => {
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input).unwrap();
            println!("stdout:{input}");
            eprintln!("stderr:{input}");
        }
        _ => {}
    }
}
