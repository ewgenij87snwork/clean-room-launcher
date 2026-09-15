#[path = "../../src/cli/screen.rs"]
#[allow(dead_code)]
mod screen;

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

struct TempProject(PathBuf);

impl TempProject {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let project =
            std::env::temp_dir().join(format!("clroom-{label}-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&project).unwrap();
        Self(project)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn readme_launch_plaque_matches_the_runtime_renderer() {
    // Break caught: README geometry, package version, labels, or selected-skill
    // counts drift from the exact plaque rendered by the shipped CLI.
    let project = TempProject::new("readme-plaque");
    for name in ["planning", "review"] {
        let skill = project.path().join(".agents/skills").join(name);
        fs::create_dir_all(&skill).unwrap();
        fs::write(skill.join("SKILL.md"), format!("# {name}\n")).unwrap();
    }

    let rendered = screen::render_isolated_preview_for(
        project.path(),
        3,
        screen::PlaqueFeatureState::default(),
        screen::RenderContext {
            width: 100,
            interactive: true,
            plain: true,
        },
    )
    .into_iter()
    .skip(3)
    .take_while(|line| !line.is_empty())
    .collect::<Vec<_>>()
    .join("\n");

    let readme = include_str!("../../README.md");
    let readme_plaque = readme
        .split_once("```text\n")
        .expect("README launch plaque must start with a text fence")
        .1
        .split_once("\n```")
        .expect("README launch plaque must end with a text fence")
        .0;

    assert_eq!(
        readme_plaque, rendered,
        "README launch plaque drifted from the runtime renderer; regenerate the example from screen::render_isolated_preview_for"
    );
}
