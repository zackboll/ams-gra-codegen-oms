//! Independently authored Task 062 corpus, not production-matcher expectations.
//! Oracle uses separator counts, not the production prefix/repetition scanner.
use std::collections::BTreeMap;

fn octet(text: &str) -> bool {
    // Literal alternatives: 25[0-5], 2[0-4][0-9], [01]?[0-9]?[0-9].
    let b = text.as_bytes();
    match b {
        [a] => a.is_ascii_digit(),
        [a, b] => a.is_ascii_digit() && b.is_ascii_digit(),
        [b'0' | b'1', b, c] => b.is_ascii_digit() && c.is_ascii_digit(),
        [b'2', b'0'..=b'4', c] => c.is_ascii_digit(),
        [b'2', b'5', b'0'..=b'5'] => true,
        _ => false,
    }
}

pub fn independent_accepts(text: &str) -> bool {
    if !(2..=45).contains(&text.len()) || !text.is_ascii() {
        return false;
    }
    let groups: Vec<_> = text.split(':').collect();
    let colons = groups.len() - 1;
    let hex = |s: &str| s.len() <= 4 && s.bytes().all(|b| b.is_ascii_hexdigit());
    if text.contains('.') {
        let Some(last) = groups.last() else {
            return false;
        };
        let decimal: Vec<_> = last.split('.').collect();
        return decimal.len() == 4
            && decimal.iter().all(|s| octet(s))
            && groups[..groups.len() - 1].iter().all(|s| hex(s))
            && ((1..=6).contains(&colons) || (colons == 7 && text.starts_with("::")));
    }
    groups.iter().all(|s| hex(s))
        && ((1..=7).contains(&colons)
            || (colons == 8 && (text.starts_with("::") || text.ends_with("::")))
            || (colons == 9 && text.starts_with("::") && text.ends_with("::")))
}

pub fn cases() -> Vec<(String, bool)> {
    let mut result = BTreeMap::new();
    let mut add = |text: String, expected: bool| {
        if let Some(previous) = result.insert(text.clone(), expected) {
            assert_eq!(
                previous, expected,
                "independent expectations conflict: {text:?}"
            );
        }
    };
    for text in [
        "::",
        "::1",
        "1::",
        "2001:db8::1",
        "2001:db8:0:0:0:0:2:1",
        "2001:db8::2:1",
        "fe80::1",
        "FE80::ABCD",
        "fE80::aBcD",
        "1:2",
        "1:",
        ":1",
        ":::",
        "1::2::3",
        ":::::::::",
        "::ffff:192.0.2.128",
        ":001.009.099.199",
        "1:255.255.255.255",
        "ffff:ffff:ffff:ffff:ffff:ffff:255.255.255.255",
    ] {
        add(text.into(), true);
    }
    for text in [
        "",
        ":",
        "1",
        "1234",
        "12345::",
        "::12345",
        "::::::::::",
        "1:2:3:4:5:6:7:8:9",
        "1:2:3:4:5:6:7:8:",
        "1:2:3:4:5:6:7:8:9:",
        "::ffff:256.0.2.128",
        "::ffff:999.0.2.128",
        "::192.0.2",
        "::192.0.2.128.1",
        "::.192.0.2.128",
        "::192.0.2.128.",
        "::192..2.128",
        "::+1.0.2.128",
        "::-1.0.2.128",
        ":: 1.0.2.128",
        "::١.0.2.128",
        "::１２.0.2.128",
        "::Ｇ",
        "::Ａ",
        "::1junk",
        " ::1",
        "::1 ",
        "::\t",
        "::\n",
        "::\r",
        "::\0",
        "::\u{7f}",
        "[::1]",
        "fe80::1%eth0",
        "2001:db8::/32",
        "ffff:ffff:ffff:ffff:ffff:fffff:255.255.255.255",
    ] {
        add(text.into(), false);
    }
    // All decimal spellings of length 1..3 (1110), in each octet position.
    // These exercise every authored alternative and every leading-zero form.
    for digits in 1..=3 {
        for n in 0..10_u32.pow(digits) {
            let spelling = format!("{n:0width$}", width = digits as usize);
            for position in 0..4 {
                let mut components = ["0", "9", "10", "255"];
                components[position] = &spelling;
                add(format!("::{}", components.join(".")), n <= 255);
            }
        }
    }
    // Every empty/nonempty hex-group partition for 1..10 separators, and
    // hex width/case boundaries. The separator formula is derived in docs.
    for colons in 1..=10 {
        for mask in 0..(1_u32 << (colons + 1)) {
            for group in ["a", "AB", "aB0", "Ab09", "ABCDE"] {
                let text = (0..=colons)
                    .map(|i| if mask & (1 << i) == 0 { "" } else { group })
                    .collect::<Vec<_>>()
                    .join(":");
                let structural = colons <= 7
                    || (colons == 8 && (mask & 3 == 0 || mask >> (colons - 1) == 0))
                    || (colons == 9 && mask & 3 == 0 && mask >> (colons - 1) == 0);
                add(
                    text.clone(),
                    text.len() >= 2
                        && text.len() <= 45
                        && structural
                        && (group.len() <= 4 || mask == 0),
                );
            }
        }
    }
    // Embedded-IPv4 colon/group boundaries, including atypical empty pieces.
    for colons in 1..=8 {
        for mask in 0..(1_u32 << colons) {
            for decimal in [
                "0.9.10.99",
                "100.199.200.249",
                "250.255.001.009",
                "256.0.0.0",
            ] {
                let prefix = (0..colons)
                    .map(|i| if mask & (1 << i) == 0 { "" } else { "Ff00" })
                    .collect::<Vec<_>>()
                    .join(":");
                let text = format!("{prefix}:{decimal}");
                add(
                    text.clone(),
                    text.len() <= 45
                        && (colons <= 6 || (colons == 7 && mask & 3 == 0))
                        && !decimal.starts_with("256"),
                );
            }
        }
    }
    result.into_iter().collect()
}
