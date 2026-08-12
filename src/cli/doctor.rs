use std::path::Path;
use taskseal::contracts::adapter::parse_declaration;
use taskseal::core::installation::verify_installation;

pub fn run(args: &[String]) -> Result<String, String> {
    let root = parse_root(args)?;
    let root = Path::new(&root);
    if !root
        .join("schemas/contracts/adapter-declaration.schema.json")
        .is_file()
    {
        return Err("DOCTOR_SCHEMA_INVALID: restore the installation schema".to_owned());
    }
    for provider in ["codex", "claude"] {
        let path = root.join(format!("adapters/declarations/{provider}.toml"));
        let text = std::fs::read_to_string(path)
            .map_err(|_| "DOCTOR_CONFIG_INVALID: restore adapter declarations".to_owned())?;
        parse_declaration(&text)
            .map_err(|_| "DOCTOR_CONFIG_INVALID: restore adapter declarations".to_owned())?;
    }
    let integrity = verify_installation(root);
    if matches!(
        integrity.code,
        "DOCTOR_ARTIFACT_INVALID" | "DOCTOR_ROOT_INVALID"
    ) {
        return Err(format!("{}: {}", integrity.code, integrity.safe_action));
    }
    Ok(format!(
        "DOCTOR_PASS\n{}\nP06_REQUIRED\nprovider qualification has not run",
        integrity.code
    ))
}

fn parse_root(args: &[String]) -> Result<String, String> {
    match args {
        [flag, root] if flag == "--root" => Ok(root.clone()),
        _ => Err("NON_INTERACTIVE_INPUT_REQUIRED: use doctor --root <directory>".to_owned()),
    }
}
