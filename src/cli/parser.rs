pub use super::help::Command;

pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Command, String> {
    let Some(first) = args.into_iter().next() else {
        return Ok(Command::Guided);
    };

    super::help::resolve(&first)
        .map(|spec| spec.command)
        .ok_or_else(|| format!("UNKNOWN_COMMAND: {first}; try help"))
}
