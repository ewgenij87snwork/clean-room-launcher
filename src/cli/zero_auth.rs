pub(crate) const ARGUMENT_REFUSAL: &str =
    "ZERO_AUTH_ARGUMENT_REFUSAL: sensitive argument refused before dispatch; continue locally";

pub(crate) fn is_sensitive_argument(value: &str) -> bool {
    let value = value.as_bytes();
    starts_with_ascii_case(value, b"sk-")
        || starts_with_ascii_case(value, b"bearer ")
        || [
            b"--token".as_slice(),
            b"--with-access-token".as_slice(),
            b"--access-token".as_slice(),
            b"--api-key".as_slice(),
            b"--api_key".as_slice(),
            b"--password".as_slice(),
            b"--secret".as_slice(),
        ]
        .iter()
        .any(|flag| {
            equals_ascii_case(value, flag)
                || value
                    .get(flag.len())
                    .is_some_and(|separator| *separator == b'=')
                    && starts_with_ascii_case(value, flag)
        })
        || [
            b"token=".as_slice(),
            b"token:".as_slice(),
            b"api-key=".as_slice(),
            b"api_key=".as_slice(),
            b"password=".as_slice(),
            b"secret=".as_slice(),
        ]
        .iter()
        .any(|needle| contains_ascii_case(value, needle))
}

fn equals_ascii_case(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
}

fn starts_with_ascii_case(value: &[u8], prefix: &[u8]) -> bool {
    value
        .get(..prefix.len())
        .is_some_and(|start| equals_ascii_case(start, prefix))
}

fn contains_ascii_case(value: &[u8], needle: &[u8]) -> bool {
    value
        .windows(needle.len())
        .any(|window| equals_ascii_case(window, needle))
}
