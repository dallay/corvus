use anyhow::Context;
use std::collections::BTreeMap;

pub fn validate_utf8_text(bytes: &[u8]) -> anyhow::Result<&str> {
    std::str::from_utf8(bytes).context("file contents are not valid UTF-8")
}

pub fn trigram_counts(bytes: &[u8]) -> BTreeMap<[u8; 3], u32> {
    let mut counts = BTreeMap::new();
    for window in bytes.windows(3) {
        let trigram = [window[0], window[1], window[2]];
        *counts.entry(trigram).or_insert(0) += 1;
    }
    counts
}
