#[path = "../src/cli/screen.rs"]
#[allow(dead_code)]
mod screen;

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

struct TempProject(PathBuf);

impl TempProject {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "clroom-readme-plaque-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
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

fn readme_plaque() -> String {
    let readme = include_str!("../README.md");
    let section = readme
        .split_once("## See the clean launch as Codex starts")
        .expect("README clean-launch section")
        .1;
    let fenced = section
        .split_once("```text\n")
        .expect("README launch plaque code fence")
        .1;
    fenced
        .split_once("\n```")
        .expect("README launch plaque closing fence")
        .0
        .to_owned()
}

#[test]
fn readme_launch_plaque_is_exactly_derived_from_the_renderer() {
    // The README example must never hand-maintain frame width or version text.
    // If renderer geometry or CARGO_PKG_VERSION changes, this test forces the
    // documented plaque to be regenerated from the same formula before merge.
    let project = TempProject::new();
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
    .join("\n");
    let rendered = rendered.trim_matches('\n');

    assert_eq!(readme_plaque(), rendered);
}
