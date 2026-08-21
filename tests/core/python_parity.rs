use crate::core::budget::{BudgetInput, BudgetLimits};
use crate::core::operations::CoreService;
use serde_json::{Value, json};
use std::io::Write;
use std::process::{Command, Stdio};

fn python(request: &Value) -> Value {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/contracts/python-reference/reference.py");
    let mut child = Command::new("python3")
        .arg("-B")
        .arg(script)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("python3 reference must be available on this qualified lane");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(request).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    serde_json::from_slice(&output.stdout).unwrap()
}

fn rust(request: &Value) -> Value {
    let layers = request.get("layers").and_then(Value::as_array);
    let input = match layers {
        Some(values) if values.len() == 3 && values.iter().all(Value::is_string) => {
            BudgetInput::from_layers(
                values[0].as_str().unwrap(),
                values[1].as_str().unwrap(),
                values[2].as_str().unwrap(),
                request["records"].as_u64().unwrap(),
                request["protected"].as_bool().unwrap(),
            )
        }
        _ => BudgetInput::new(
            "missing-layers",
            request["records"].as_u64().unwrap(),
            request["protected"].as_bool().unwrap(),
        ),
    };
    let limits = BudgetLimits::new(
        request["limits"]["bytes"].as_u64().unwrap(),
        request["limits"]["records"].as_u64().unwrap(),
        request["limits"]["tokens"].as_u64().unwrap(),
    );
    match CoreService::compile(
        input,
        limits,
        request["inputs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_owned())
            .collect(),
    ) {
        Ok(mut report) => {
            let context = report.artifacts.remove("context.md").unwrap();
            json!({
                "status": "ok",
                "context_hex": hex(&context),
                "manifest": report.manifest
            })
        }
        Err(error) => json!({
            "status": "refused",
            "code": error.decision.reason_code
        }),
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn privacy_clean_python_reference_matches_rust_bytes_manifests_and_refusals() {
    let cases = [
        json!({"layers":["safe","project","task"],"records":3,"protected":true,"limits":{"bytes":100,"records":3,"tokens":100},"inputs":["scope:z","scope:a"]}),
        json!({"layers":["α\n\n","проєкт","任務\r\n"],"records":3,"protected":false,"limits":{"bytes":100,"records":3,"tokens":100},"inputs":["scope:unicode"]}),
        json!({"layers":["too large","",""],"records":1,"protected":false,"limits":{"bytes":1,"records":1,"tokens":100},"inputs":[]}),
        json!({"layers":["x","y","z"],"records":4,"protected":true,"limits":{"bytes":100,"records":3,"tokens":100},"inputs":[]}),
        json!({"records":1,"protected":false,"limits":{"bytes":100,"records":1,"tokens":100},"inputs":[]}),
    ];
    for case in cases {
        let reference = python(&case);
        let mut actual = rust(&case);
        if std::env::var_os("TASKSEAL_PARITY_INJECT_MISMATCH").is_some() {
            actual["status"] = json!("known-mismatch");
        }
        assert_eq!(actual, reference, "case: {case}");
    }
}
