use std::{fs, path::Path};

fn read(path: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path)).unwrap()
}

#[test]
fn discovery_surfaces_keep_the_canonical_namespace_and_crawler_access() {
    let config = read("docs/_config.yml");
    assert!(config.contains("title: Clean Room Launcher (CLROOM)"));
    assert!(config.contains("url: \"https://y-sor.github.io\""));
    assert!(config.contains("baseurl: \"/clean-room-launcher\""));
    assert!(config.contains("repository: y-sor/clean-room-launcher"));

    let sitemap = read("docs/sitemap.xml");
    assert!(sitemap.contains("permalink: /sitemap.xml"));
    assert!(sitemap.contains("page.url | absolute_url"));
    assert!(sitemap.contains("page.sitemap == false"));

    let robots = read("docs/robots.txt");
    for agent in ["Googlebot", "Bingbot", "OAI-SearchBot", "Claude-SearchBot", "Claude-User"] {
        assert!(robots.contains(&format!("User-agent: {agent}")), "missing crawler policy for {agent}");
    }
    assert!(robots.contains("Sitemap: https://y-sor.github.io/clean-room-launcher/sitemap.xml"));

    let llms = read("docs/llms.txt");
    assert!(llms.contains("# Clean Room Launcher (CLROOM)"));
    assert!(llms.contains("https://github.com/y-sor/clean-room-launcher"));
    assert!(llms.contains("CLROOM is the acronym for **Clean Room Launcher**"));

    for surface in [&config, &sitemap, &robots, &llms] {
        assert!(!surface.contains("ewgenij87snwork.github.io/clean-room-launcher"));
        assert!(!surface.contains("github.com/ewgenij87snwork/clean-room-launcher"));
    }
}

#[test]
fn homepage_metadata_exposes_analytics_preview_and_free_app_facts() {
    let head = read("docs/_includes/head.html");
    assert!(head.contains("max-image-preview:large"));
    assert!(head.contains("\"@type\": \"SoftwareApplication\""));
    assert!(head.contains("\"price\": 0"));
    assert!(head.contains("static.cloudflareinsights.com/beacon.min.js"));
    assert!(head.contains("a18fd1827d4c48d2a22277f14eade9b2"));

    let home = read("docs/index.md");
    assert!(home.contains("title: Clean Room Launcher (CLROOM)"));
    assert!(home.contains("path: /assets/clean-room-launcher-hero.png"));
    assert!(home.contains("the intended public identity is **Clean Room Launcher (CLROOM)**"));
}
