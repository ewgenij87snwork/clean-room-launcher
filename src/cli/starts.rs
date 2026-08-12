use super::{
    consent::{Decision, LaunchConsent, LaunchProposal},
    state::{AccessClass, SavedStart, StateStore},
};

pub fn run(args: &[String]) -> Result<String, String> {
    let store = StateStore::for_current_user().map_err(|error| error.code().to_owned())?;
    let saved = store.load().map_err(|error| error.code().to_owned())?;
    match args.first().map(String::as_str) {
        Some("starts") if args.len() == 1 => Ok(render(&saved.starts)),
        Some("start") => preflight(&saved.starts, args),
        _ => Err("SAVED_START_USAGE: use starts or start <n> --approve".to_owned()),
    }
}

fn render(starts: &[SavedStart]) -> String {
    if starts.is_empty() {
        return "NO_SAVED_STARTS".to_owned();
    }
    starts
        .iter()
        .enumerate()
        .map(|(index, start)| {
            let access = match start.access_class {
                AccessClass::Standard => "STANDARD",
                AccessClass::FullAccess => "FULL ACCESS",
            };
            format!("{}. {} · {access}", index + 1, start.provider)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn preflight(starts: &[SavedStart], args: &[String]) -> Result<String, String> {
    let Some(index) = args.get(1).and_then(|value| value.parse::<usize>().ok()) else {
        return Err("SAVED_START_USAGE: use start <n> --approve".to_owned());
    };
    let Some(start) = index.checked_sub(1).and_then(|index| starts.get(index)) else {
        return Err("SAVED_START_NOT_FOUND".to_owned());
    };
    if start.access_class == AccessClass::FullAccess {
        return Err("FULL ACCESS: FULL_ACCESS_CHOOSER_REQUIRED".to_owned());
    }
    let proposal = LaunchProposal::new(
        start.project_digest.clone(),
        Vec::new(),
        Vec::new(),
        start.provider.clone(),
        taskseal::core::inventory::sha256_hex(&start.argv.join("\0").into_bytes()),
        start.qualification_digest.clone(),
        start.access_class.clone(),
    )
    .map_err(|error| error.code().to_owned())?;
    let decision = if args.get(2).map(String::as_str) == Some("--approve") && args.len() == 3 {
        Decision::Approve
    } else {
        Decision::Cancel
    };
    let consent =
        LaunchConsent::resolve(decision, &proposal).map_err(|error| error.code().to_owned())?;
    consent
        .verify(&proposal)
        .map_err(|error| error.code().to_owned())?;
    Err("P06_REQUIRED: provider launch is not qualified".to_owned())
}
