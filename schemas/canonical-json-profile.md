# Restricted canonical JSON profile

TaskSeal canonical bytes use compact UTF-8 JSON. Object keys are ordered by
their Unicode scalar string order recursively; arrays preserve order. No
whitespace is emitted. Values must be representable by `serde_json::Value` and
must not use an unsupported non-finite number. Merge behavior is selected only
through the typed `MergeOperation` enum; recursive implicit deep merge is not a
public operation.
