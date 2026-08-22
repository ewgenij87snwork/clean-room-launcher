#[path = "../../src/cli/screen.rs"]
#[allow(dead_code)]
mod screen;

use std::path::Path;

#[test]
fn isolated_preview_renders_the_accepted_plain_launch_receipt() {
    // Break caught: the approved receipt geometry, spacing, or boundary claims drift.
    let output = screen::render_isolated_preview_for(
        Path::new("/private/tmp/clroom-preview-project"),
        0,
        screen::RenderContext {
            width: 100,
            interactive: true,
            plain: true,
        },
    )
    .join("\n");

    assert_eq!(
        output,
        "\n\n\n\
╓──○──╖ ╭─ CLEAN ROOM ──────────────────╮\n\
║░░░░░║⠒│                               │\n\
║░░░░░║⠒│     Global AGENTS.md  off     │\n\
║░░░░░║⠒│     Global skills     off     │\n\
║░░░░░║⠒│     Apps              off     │\n\
║░░░░░║⠒│     Hooks/plugins     off     │\n\
║░░░░░║⠒│     Dev prompt        off     │\n\
║░░░░░║⠒│     Notifications     off     │\n\
║░░░░░║⠒│                               │\n\
╙──○──╜ ╰───────────────────────────────╯\n"
    );
}

#[test]
fn isolated_preview_styles_only_the_visual_hierarchy() {
    // Break caught: ANSI styling changes the receipt text or its five-cell inset.
    let output = screen::render_isolated_preview_for(
        Path::new("/tmp/project"),
        0,
        screen::RenderContext {
            width: 100,
            interactive: true,
            plain: false,
        },
    )
    .join("\n");

    assert!(output.starts_with("\n\n\n\u{1b}[2m╓──○──╖\u{1b}[0m "));
    assert!(output.contains("\u{1b}[1;36mCLEAN ROOM\u{1b}[0m"));
    assert!(
        output.contains("\u{1b}[2m║░░░░░║⠒│\u{1b}[0m     \u{1b}[1mGlobal AGENTS.md\u{1b}[0m  off")
    );
    assert!(!output.contains("/tmp/project"));
}

#[test]
fn isolated_preview_reports_the_number_of_selected_global_skills() {
    // Break caught: the plaque claims every global skill is off even though
    // this one launch admitted an explicit selection.
    let output = screen::render_isolated_preview_for(
        Path::new("/tmp/project"),
        12,
        screen::RenderContext {
            width: 100,
            interactive: true,
            plain: true,
        },
    )
    .join("\n");

    assert!(output.contains("     Global skills   12 on     "));
    assert!(!output.contains("Global skills     off"));
}
