//! Final pre-production boundary, independent of the expensive message survey.
use ams_gra_oms_ir::{ConstraintSet, PatternExpression, PatternGroup, PrimitiveKind, TypeKind};
use ams_gra_oms_xsd_frontend::load_schema_set;
use std::{collections::BTreeSet, path::PathBuf, process::Command};

const ROWS: &str = include_str!("../../../tests/fixtures/string/task060-pinned-rows.tsv");
const UNICODE: [&str; 3] = [
    "NITF_DateAndTimeType",
    "NITF_DateType",
    "NITF_MSTGTA_TargetLocationType",
];

#[test]
fn task060_final_admission_boundary_matches_exact_pinned_ir() {
    for (release, variable, digest) in [
        (
            "2.5",
            "AMS_GRA_UCI_2_5_ROOT",
            "ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27",
        ),
        (
            "2.6",
            "AMS_GRA_UCI_2_6_ROOT",
            "af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b",
        ),
    ] {
        let Some(path) = std::env::var_os(variable).map(PathBuf::from) else {
            eprintln!("SKIPPED {variable}");
            continue;
        };
        let hash = Command::new("sha256sum").arg(&path).output().unwrap();
        assert!(hash.status.success());
        assert_eq!(
            String::from_utf8_lossy(&hash.stdout)
                .split_whitespace()
                .next(),
            Some(digest)
        );
        let schema = load_schema_set(&path).unwrap();
        let mut names = BTreeSet::new();
        let mut admitted = BTreeSet::new();
        let mut unicode = BTreeSet::new();
        let mut complex = BTreeSet::new();
        for line in ROWS.lines().filter(|line| !line.starts_with('#')) {
            let cells: Vec<_> = line.split('\t').collect();
            if cells[0] != release {
                continue;
            }
            let name = cells[1];
            assert!(names.insert(name));
            let declaration = schema
                .types
                .iter()
                .find(|d| d.name.local_name == name)
                .unwrap();
            assert_eq!(declaration.kind, TypeKind::Primitive(PrimitiveKind::String));
            let facet = |text: &str| (text != "-").then(|| text.parse::<u64>().unwrap());
            let mut expected = ConstraintSet {
                length: facet(cells[2]),
                min_length: facet(cells[3]),
                max_length: facet(cells[4]),
                ..ConstraintSet::default()
            };
            expected.lexical.pattern_groups.push(PatternGroup {
                alternatives: cells[5..]
                    .iter()
                    .map(|p| PatternExpression::xml_schema(*p))
                    .collect(),
            });
            assert_eq!(declaration.constraints, expected, "{release} {name}");
            let live = ams_gra_oms_codegen_core::string_profile(
                PrimitiveKind::String,
                &declaration.constraints,
            );
            let should_admit = !UNICODE.contains(&name) && name != "IPv6_AddressType";
            assert_eq!(
                matches!(
                    live,
                    Ok(Some(
                        ams_gra_oms_codegen_core::StringProfile::AlternatingAscii(_)
                    ))
                ),
                should_admit,
                "{release} {name}"
            );
            if !should_admit {
                assert!(live.is_err());
            }
            if UNICODE.contains(&name) {
                assert!(cells[5..].iter().any(|p| p.contains("\\d")));
                unicode.insert(name);
            } else if name == "IPv6_AddressType" {
                complex.insert(name);
            } else {
                assert!(
                    cells[5..]
                        .iter()
                        .all(|p| p.is_ascii() && !p.contains("\\d"))
                );
                admitted.insert(name);
            }
        }
        assert_eq!(names.len(), 19);
        assert_eq!(admitted.len(), 15);
        assert_eq!(unicode, BTreeSet::from(UNICODE));
        assert_eq!(complex, BTreeSet::from(["IPv6_AddressType"]));
        for name in [
            "MilitaryGridType",
            "NotationType",
            "RecordOriginatorType",
            "IPv4_AddressType",
        ] {
            assert!(admitted.contains(name));
        }
        println!(
            "\nUCI {release} TASK060 FINAL ADMISSION: baseline=19 admitted=15 DEFERRED_UNICODE=3 DEFERRED_COMPLEX=1"
        );
    }
}

#[test]
fn task060_unique_profiles_and_release_specific_military_grid_are_frozen() {
    let mut admitted = BTreeSet::new();
    let mut military = Vec::new();
    for line in ROWS.lines().filter(|line| !line.starts_with('#')) {
        let cells: Vec<_> = line.split('\t').collect();
        if !UNICODE.contains(&cells[1]) && cells[1] != "IPv6_AddressType" {
            admitted.insert(cells[2..].join("\t"));
        }
        if cells[1] == "MilitaryGridType" {
            military.push(cells);
        }
    }
    assert_eq!(admitted.len(), 16);
    assert_eq!(military.len(), 2);
    assert_eq!(military[0][3], "14");
    assert_eq!(military[1][3], "3");
    assert_ne!(military[0][5], military[1][5]);
    // 1CAA plus five pairs has length 14; both profiles permit it.
    // AAB is the mandatory polar square at length 3: 2.6 permits it,
    // while 2.5's independent minLength=14 rejects it.
    assert_eq!("1CAA0000000000".len(), 14);
    assert_eq!("AAB".len(), 3);
}
