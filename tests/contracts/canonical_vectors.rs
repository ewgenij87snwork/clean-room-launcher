use serde_json::json;
use taskseal::contracts::canonical::{MergeOperation, canonicalize, merge};

#[test]
fn canonical_bytes_are_key_sorted_compact_and_repeatable() {
    let value = json!({"z": [2, 1], "a": "тест", "nested": {"b": true, "a": null}});
    let first = canonicalize(&value).unwrap();
    let second = canonicalize(&value).unwrap();
    assert_eq!(first, second);
    assert_eq!(
        first,
        "{\"a\":\"тест\",\"nested\":{\"a\":null,\"b\":true},\"z\":[2,1]}".as_bytes()
    );
}

#[test]
fn merge_operations_are_typed_and_recursive_deep_merge_is_not_available() {
    let left = json!({"items":["a"],"mode":"old"});
    let right = json!({"items":["a","b"]});
    assert_eq!(
        merge(&left, &right, MergeOperation::Replace).unwrap(),
        right
    );
    assert_eq!(
        merge(&left, &right, MergeOperation::AppendUnique).unwrap(),
        json!({"items":["a","b"],"mode":"old"})
    );
    assert!(merge(&left, &right, MergeOperation::NoInherit).is_err());
}

#[test]
fn ambiguous_merge_and_non_object_inputs_refuse() {
    let left = json!({"a":1});
    let right = json!({"a":2});
    assert!(merge(&left, &right, MergeOperation::DenyUnion).is_err());
    assert!(merge(&json!(1), &right, MergeOperation::MapOverride).is_err());
}
