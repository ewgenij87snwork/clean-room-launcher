#[path = "../../src/cli/screen.rs"]
#[allow(dead_code)]
mod screen;

use std::path::Path;

#[test]
fn isolated_preview_names_the_actual_project_boundary_and_explicit_launch() {
    // Break caught: the visible launch screen is static or implies changed provider state.
    let project = Path::new("/private/tmp/clroom-preview-project");
    let output = screen::render_isolated_preview(project, "Codex").join("\n");

    assert!(output.contains("Project  /private/tmp/clroom-preview-project"));
    assert!(output.contains("Boundary active · global AGENTS.md and ambient skills excluded"));
    assert!(output.contains("Defaults hooks/plugins off · explicit user overrides win"));
    assert!(output.contains(
        "Notice   Codex may show `Operation not permitted` for blocked ambient files; expected"
    ));
    assert!(output.contains("Provider Codex · existing provider state stays untouched"));
    assert!(output.contains("Action   Launch Codex"));
    assert!(!output.contains("Launch succeeded"));
}
