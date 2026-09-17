use clroom::core::manifest::{Compilation, build_manifest};
use clroom::core::render::ArtifactSet;

#[test]
fn generation_digest_frames_concatenation_ambiguous_input_tuples() {
    let artifacts = ArtifactSet::from([("context.md".to_owned(), b"stable".to_vec())]);

    let left = Compilation::new(
        vec!["ab".to_owned(), "c".to_owned()],
        artifacts.clone(),
    );
    let right = Compilation::new(vec!["a".to_owned(), "bc".to_owned()], artifacts);

    let left = build_manifest(&left).unwrap();
    let right = build_manifest(&right).unwrap();

    assert_ne!(left.inputs, right.inputs);
    assert_ne!(left.digest, right.digest);
}
