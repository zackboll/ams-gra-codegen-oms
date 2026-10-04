//! Shared semantic cases for standalone backend probes, before capability wiring.
use ams_gra_oms_codegen_core::{
    AlternatingAsciiProfile, StructuredAsciiFacets, StructuredAsciiRepetition,
    StructuredAsciiSegment, alternating_ascii_rows,
};

pub fn profiles() -> Vec<(String, AlternatingAsciiProfile)> {
    let mut profiles: Vec<_> = alternating_ascii_rows()
        .iter()
        .enumerate()
        .map(|(i, row)| (format!("Carrier{i}"), row.profile.clone()))
        .collect();
    let branch = |s| vec![StructuredAsciiSegment::Literal(s)];
    profiles.push((
        "Intersection".into(),
        AlternatingAsciiProfile {
            facets: StructuredAsciiFacets::Exact(2),
            groups: vec![
                vec![branch("AA"), branch("BB")],
                vec![branch("BB"), branch("CC")],
            ],
        },
    ));
    profiles
}

pub fn cases(profile: &AlternatingAsciiProfile) -> Vec<(String, bool)> {
    let mut texts = vec![
        String::new(),
        "AA".into(),
        "BB".into(),
        "CC".into(),
        "DD".into(),
    ];
    for branch in profile.groups.iter().flatten() {
        let mut text = String::new();
        for segment in branch {
            match segment {
                StructuredAsciiSegment::Literal(s) => text.push_str(s),
                StructuredAsciiSegment::Class(alphabet, StructuredAsciiRepetition::Exact(n)) => {
                    text.extend(std::iter::repeat_n(
                        char::from(alphabet.ranges()[0].0),
                        usize::from(*n),
                    ));
                }
                _ => panic!("unexpanded Task060 branch"),
            }
        }
        texts.push(text.clone());
        texts.push(format!("!{text}"));
        texts.push(format!("{text}!"));
        if !text.is_empty() {
            texts.push(text[..text.len() - 1].to_owned());
        }
        texts.push(text.to_ascii_lowercase());
        for bad in ['\t', '\n', '\r', '\0', '\u{7f}', '١'] {
            texts.push(format!("{bad}{text}"));
        }
    }
    for text in [
        "ABC12",
        "UNKN",
        "NONE",
        "UNKN0",
        "NONE0",
        "UNKX",
        "E",
        "-",
        "--",
        "E-",
        "AAB",
        "1CAA0000000000",
        "9.99.199.249",
        "1..2.3",
        "١.2.3.4",
        " 1.2.3.4",
        "0.0.0.0",
        "1.2.3.4",
        "10.20.30.40",
        "255.255.255.255",
        "256.0.0.1",
        "999.1.1.1",
        "01.2.3.4",
        "1.2.3",
        "1.2.3.4.5",
        ".1.2.3.4",
        "1.2.3.4.",
        "+1.2.3.4",
    ] {
        texts.push(text.into());
    }
    texts.sort();
    texts.dedup();
    texts
        .into_iter()
        .map(|text| {
            let valid = profile.accepts(text.as_bytes());
            (text, valid)
        })
        .collect()
}
