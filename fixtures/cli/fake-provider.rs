use std::{env, fs, io::Read};

fn main() {
    let capture = env::var("CLROOM_CAPTURE_PATH").expect("capture path is required");
    let args = env::args().skip(1).collect::<Vec<_>>();
    let marker = env::var("CLROOM_INHERITED_MARKER").unwrap_or_default();
    fs::write(capture, format!("{}\0{marker}", args.join("\0"))).expect("capture must be writable");
    if args.iter().any(|argument| argument == "--exit-42") {
        std::process::exit(42);
    }
    if args.iter().any(|argument| argument == "--abort") {
        std::process::abort();
    }
    if args.iter().any(|argument| argument == "--stdio") {
        let mut input = String::new();
        std::io::stdin().read_to_string(&mut input).unwrap();
        println!("stdout:{input}");
        eprintln!("stderr:{input}");
    }
}
