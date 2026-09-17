#![no_main]

use clroom::core::manifest::{Compilation, build_manifest};
use clroom::core::render::ArtifactSet;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let quarter = data.len() / 4;
    let (a, rest) = data.split_at(quarter);
    let (b, rest) = rest.split_at(quarter.min(rest.len()));
    let (c, d) = rest.split_at(quarter.min(rest.len()));

    let input_a = String::from_utf8_lossy(a).into_owned();
    let input_b = String::from_utf8_lossy(b).into_owned();

    let mut first_artifacts = ArtifactSet::new();
    first_artifacts.insert("a.bin".to_owned(), c.to_vec());
    first_artifacts.insert("b.bin".to_owned(), d.to_vec());

    let mut second_artifacts = ArtifactSet::new();
    second_artifacts.insert("b.bin".to_owned(), d.to_vec());
    second_artifacts.insert("a.bin".to_owned(), c.to_vec());

    let first = Compilation::new(
        vec![input_a.clone(), input_b.clone()],
        first_artifacts,
    );
    let second = Compilation::new(vec![input_b, input_a], second_artifacts);

    let first = build_manifest(&first).expect("fuzz target always supplies artifacts");
    let second = build_manifest(&second).expect("fuzz target always supplies artifacts");

    assert_eq!(first.inputs, second.inputs);
    assert_eq!(first.outputs, second.outputs);
    assert_eq!(first.digest, second.digest);
});
