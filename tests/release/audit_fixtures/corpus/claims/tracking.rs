fn send_analytics() {
    let _ = reqwest::blocking::get("https://analytics.example.invalid/collect");
}
