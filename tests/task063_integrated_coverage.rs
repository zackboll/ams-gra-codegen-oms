//! Integrated-current oracle; the Task 062/063/065 isolated ledgers remain frozen.
use ams_gra_oms_codegen_core::GenerationWorld;

/// Compose frozen independent campaigns without rewriting either measurement.
pub fn integrated_coverage(release: &str, world: GenerationWorld, backend: &str) -> Vec<usize> {
    let (world_key, current_world_key) = match world {
        GenerationWorld::ClosedSchemaSet => ("closed-schema", "ClosedSchemaSet"),
        GenerationWorld::OpenExtensions => ("open-extensions", "OpenExtensions"),
    };
    let isolated = include_str!("fixtures/integral/task063-coverage.tsv");
    let rows: Vec<_> = isolated
        .lines()
        .filter(|line| line.starts_with(&format!("{release}\t{world_key}\t{backend}\t")))
        .collect();
    assert_eq!(rows.len(), 1, "unique isolated coverage cell");
    let values: Vec<usize> = rows[0]
        .split('\t')
        .skip(3)
        .map(|value| value.parse().unwrap())
        .collect();
    assert_eq!(values.len(), 10);
    let delta: Vec<_> = values[5..]
        .iter()
        .zip(&values[..5])
        .map(|(after, before)| after.checked_sub(*before).unwrap())
        .collect();
    assert_eq!(
        delta,
        if release == "2.5" {
            vec![
                0,
                1,
                0,
                0,
                if world == GenerationWorld::ClosedSchemaSet {
                    2
                } else {
                    1
                },
            ]
        } else {
            vec![0; 5]
        },
        "Task 063 isolated per-cell delta"
    );
    // Task 065's isolated AFTER includes Task 062 and repairs only Ada naming.
    // Compose Task 063's measured independent capability delta on that ledger.
    let current = include_str!("fixtures/string/task065-current-coverage.tsv");
    let rows: Vec<_> = current
        .lines()
        .filter(|line| {
            line.starts_with(&format!(
                "COVERAGE\t{release}\t{current_world_key}\t{backend}\t"
            ))
        })
        .collect();
    assert_eq!(rows.len(), 1, "unique historical Task 065 AFTER coverage cell");
    [
        "declaration_kinds_renderable",
        "declarations_fully_renderable",
        "field_type_references_renderable",
        "field_occurrences_renderable",
        "message_closures_renderable",
    ]
    .iter()
    .zip(delta)
    .map(|(metric, gain)| {
        let value = rows[0]
            .split_once(&format!("{metric}: "))
            .unwrap()
            .1
            .split([',', ' '])
            .next()
            .unwrap()
            .parse::<usize>()
            .unwrap();
        value + gain
    })
    .collect()
}
