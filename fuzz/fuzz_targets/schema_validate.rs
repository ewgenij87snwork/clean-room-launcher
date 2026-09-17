#![no_main]

use clroom::contracts::schema::{SchemaKind, validate};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let selector = data.first().copied().unwrap_or(0);
    let payload = data.get(1..).unwrap_or_default();
    let kind = match selector % 4 {
        0 => SchemaKind::L2,
        1 => SchemaKind::TaskPacketV2,
        2 => SchemaKind::Catalog,
        _ => SchemaKind::Manifest,
    };

    let _ = validate(kind, payload);
});
