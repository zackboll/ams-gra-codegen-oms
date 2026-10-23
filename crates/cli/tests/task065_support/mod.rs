//! Test-harness scheduling only; no production capability or generation rules.
pub mod profile;

use std::{
    collections::BTreeSet,
    sync::atomic::{AtomicUsize, Ordering},
};

pub fn worker_count(value: Option<&str>) -> Result<usize, String> {
    match value {
        None => Ok(4),
        Some("1") => Ok(1),
        Some("2") => Ok(2),
        Some("4") => Ok(4),
        Some(value) => Err(format!(
            "AMS_GRA_TASK065_WORKERS must be 1, 2, or 4, got {value:?}"
        )),
    }
}

/// Check the entire ledger before returning anything to the qualification.
/// Identities and indices must both match, even if the cardinality is correct.
fn assemble<T>(
    expected: &[&str],
    results: Vec<(usize, String, Result<T, String>)>,
) -> Result<Vec<T>, String> {
    if expected.iter().copied().collect::<BTreeSet<_>>().len() != expected.len() {
        return Err("duplicate candidate input".into());
    }
    let mut slots: Vec<_> = (0..expected.len()).map(|_| None).collect();
    for (index, identity, result) in results {
        if expected.get(index) != Some(&identity.as_str()) {
            return Err(format!("out-of-set candidate result {index}: {identity}"));
        }
        if slots[index].is_some() {
            return Err(format!("duplicate candidate result {index}: {identity}"));
        }
        slots[index] = Some(result);
    }
    slots
        .into_iter()
        .enumerate()
        .map(|(index, result)| {
            result
                .ok_or_else(|| format!("missing candidate result {index}: {}", expected[index]))?
                .map_err(|error| format!("candidate {index} {}: {error}", expected[index]))
        })
        .collect()
}

/// A fixed number of scoped standard-library threads, with one live candidate
/// per worker. Workers neither print evidence nor start nested thread pools.
/// Joining every worker preserves panic propagation without detached work.
pub fn ordered_candidates<T: Send, F>(
    expected: &[&str],
    workers: usize,
    evaluate: F,
) -> Result<Vec<T>, String>
where
    F: Fn(usize, &str) -> Result<T, String> + Sync,
{
    if !matches!(workers, 1 | 2 | 4) {
        return Err(format!("invalid worker budget: {workers}"));
    }
    if expected.iter().copied().collect::<BTreeSet<_>>().len() != expected.len() {
        return Err("duplicate candidate input".into());
    }
    // Preserve a truly serial reference, including the calling thread.
    if workers == 1 {
        return assemble(
            expected,
            expected
                .iter()
                .enumerate()
                .map(|(index, name)| (index, (*name).into(), evaluate(index, name)))
                .collect(),
        );
    }
    let next = AtomicUsize::new(0);
    let results = std::thread::scope(|scope| {
        let evaluate = &evaluate;
        let next = &next;
        let handles: Vec<_> = (0..workers.min(expected.len()))
            .map(|_| {
                scope.spawn(move || {
                    let mut results = Vec::new();
                    loop {
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        let Some(name) = expected.get(index) else {
                            break;
                        };
                        results.push((index, (*name).into(), evaluate(index, name)));
                    }
                    results
                })
            })
            .collect();
        let mut results = Vec::new();
        let mut panic = None;
        for handle in handles {
            match handle.join() {
                Ok(mut rows) => results.append(&mut rows),
                Err(payload) => {
                    if panic.is_none() {
                        panic = Some(payload);
                    }
                }
            }
        }
        if let Some(payload) = panic {
            std::panic::resume_unwind(payload);
        }
        results
    });
    assemble(expected, results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn task065_scheduler_budget_controls() {
        assert_eq!(worker_count(None).unwrap(), 4);
        for count in [1, 2, 4] {
            assert_eq!(worker_count(Some(&count.to_string())).unwrap(), count);
        }
        for value in ["", "0", "3", "5", "256", "-1", " 2", "2 ", "01", "no"] {
            assert!(worker_count(Some(value)).is_err(), "{value}");
        }
        for count in [0, 3, 5, usize::MAX] {
            assert!(ordered_candidates::<(), _>(&["a"], count, |_, _| Ok(())).is_err());
        }
    }

    #[test]
    fn task065_scheduler_order_completeness_and_bound() {
        let names: Vec<_> = (0..66).map(|i| format!("message-{i}")).collect();
        let expected: Vec<_> = names.iter().map(String::as_str).collect();
        for workers in [1, 2, 4] {
            let active = AtomicUsize::new(0);
            let peak = AtomicUsize::new(0);
            let counts: Vec<_> = (0..66).map(|_| AtomicUsize::new(0)).collect();
            let result = ordered_candidates(&expected, workers, |index, name| {
                let live = active.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(live, Ordering::SeqCst);
                counts[index].fetch_add(1, Ordering::SeqCst);
                // Deliberately complete later indices sooner.
                std::thread::sleep(std::time::Duration::from_millis((4 - index % 4) as u64));
                active.fetch_sub(1, Ordering::SeqCst);
                Ok((index, name.to_owned()))
            })
            .unwrap();
            assert_eq!(
                result,
                names.iter().cloned().enumerate().collect::<Vec<_>>()
            );
            assert!(counts.iter().all(|c| c.load(Ordering::SeqCst) == 1));
            assert!(peak.load(Ordering::SeqCst) <= workers);
            assert_eq!(active.load(Ordering::SeqCst), 0);
        }
        assert!(
            ordered_candidates::<(), _>(&[], 4, |_, _| unreachable!())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn task065_scheduler_rejects_corrupt_ledgers() {
        let expected = ["a", "b"];
        assert!(assemble::<()>(&expected, vec![]).is_err());
        assert!(assemble(&expected, vec![(0, "a".into(), Ok(()))]).is_err());
        assert!(
            assemble(
                &expected,
                vec![(0, "a".into(), Ok(())), (0, "a".into(), Ok(()))]
            )
            .is_err()
        );
        assert!(assemble(&expected, vec![(2, "c".into(), Ok(()))]).is_err());
        assert!(assemble(&expected, vec![(0, "b".into(), Ok(()))]).is_err());
        assert!(assemble::<()>(&["a", "a"], vec![]).is_err());
        let called = AtomicBool::new(false);
        assert!(
            ordered_candidates(&["a", "a"], 2, |_, _| {
                called.store(true, Ordering::SeqCst);
                Ok(())
            })
            .is_err()
        );
        assert!(!called.load(Ordering::SeqCst));
    }

    #[test]
    fn task065_scheduler_propagates_failures_and_panics() {
        for workers in [1, 2, 4] {
            let result = ordered_candidates(&["a", "b", "c", "d"], workers, |index, _| {
                if index == 1 {
                    Err("injected operation failure".into())
                } else {
                    Ok(())
                }
            });
            assert!(
                result
                    .unwrap_err()
                    .contains("candidate 1 b: injected operation failure")
            );
            let completed = AtomicUsize::new(0);
            let result = std::panic::catch_unwind(|| {
                ordered_candidates(&["a", "b", "c", "d"], workers, |index, _| {
                    assert_ne!(index, 1, "injected worker panic");
                    completed.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                })
            });
            assert!(result.is_err());
            let before = completed.load(Ordering::SeqCst);
            std::thread::sleep(std::time::Duration::from_millis(10));
            assert_eq!(before, completed.load(Ordering::SeqCst), "no detached work");
        }
    }
}
