//! Test-only Linux task observation. pthread join precedes final procfs removal.
use std::collections::BTreeSet;
use std::io;
use std::path::Path;

const PF_EXITING: u64 = 0x0000_0004;
const WORKER_PREFIX: &str = "ams-gra-oms-lac";

#[derive(Debug, PartialEq, Eq)]
struct Task {
    tid: u32,
    comm: String,
    state: char,
    flags: u64,
}

fn parse_stat(record: &str) -> Result<Task, String> {
    // comm can contain spaces and parentheses. Fields after its LAST closing
    // parenthesis start with state (field 3); flags is field 9, not field 8.
    let (head, tail) = record
        .rsplit_once(") ")
        .ok_or("missing stat comm terminator")?;
    let (tid, comm) = head.split_once(" (").ok_or("missing stat tid/comm")?;
    let fields: Vec<_> = tail.split_whitespace().collect();
    if fields.len() < 50 || fields[0].len() != 1 {
        return Err("incomplete stat record".into());
    }
    let state = fields[0].chars().next().unwrap();
    if !"RSDZTtXxKWPI".contains(state)
        || fields[1..]
            .iter()
            .any(|field| field.parse::<i128>().is_err())
    {
        return Err("invalid stat state/numeric metadata".into());
    }
    Ok(Task {
        tid: tid.parse().map_err(|_| "invalid stat tid")?,
        comm: comm.into(),
        state,
        flags: fields[6].parse().map_err(|_| "invalid stat flags")?,
    })
}

fn disappeared(path: &Path) -> bool {
    matches!(std::fs::metadata(path), Err(error) if error.kind() == io::ErrorKind::NotFound)
}

fn read_metadata(path: &Path, name: &str) -> Option<String> {
    match std::fs::read_to_string(path.join(name)) {
        Ok(text) => Some(text),
        Err(error) if disappeared(path) => {
            eprintln!(
                "RUST_WORKER_DISAPPEARED task={} reading={name}: {error}",
                path.display()
            );
            None
        }
        Err(error) => panic!("unreadable extant task {} {name}: {error}", path.display()),
    }
}

pub(crate) fn workers_at(directory: &Path) -> BTreeSet<String> {
    let mut workers = BTreeSet::new();
    for entry in std::fs::read_dir(directory).expect("read process task directory") {
        let entry = entry.expect("read task entry");
        let path = entry.path();
        let tid = entry
            .file_name()
            .to_str()
            .expect("non-UTF8 task id")
            .to_owned();
        let expected_tid: u32 = tid.parse().expect("non-numeric task id");
        let Some(comm) = read_metadata(&path, "comm") else {
            continue;
        };
        if !comm.starts_with(WORKER_PREFIX) {
            continue;
        }
        let Some(record) = read_metadata(&path, "stat") else {
            continue;
        };
        let task = match parse_stat(&record) {
            Ok(task) => task,
            Err(error) if disappeared(&path) => {
                eprintln!("RUST_WORKER_DISAPPEARED tid={tid}: {error}; stat={record:?}");
                continue;
            }
            Err(error) => panic!("malformed extant task {tid}: {error}; stat={record:?}"),
        };
        assert_eq!(
            task.tid, expected_tid,
            "stat task identity mismatch: {record}"
        );
        assert_eq!(
            task.comm,
            comm.trim_end_matches('\n'),
            "comm/stat mismatch: {record}"
        );
        eprintln!(
            "RUST_WORKER_OBSERVATION task={} tid={} comm={:?} state={} flags={:#x} stat={record:?}",
            path.display(),
            task.tid,
            task.comm,
            task.state,
            task.flags
        );
        // State R alone is NOT proof of a live userspace worker. do_exit sets
        // PF_EXITING before clear_child_tid wakes pthread_join's futex waiter.
        // Conversely, sleeping/blocked/runnable tasks without this flag count.
        if task.flags & PF_EXITING == 0 {
            workers.insert(tid);
        }
    }
    workers
}

pub fn application_workers(pid: u32) -> BTreeSet<String> {
    workers_at(Path::new(&format!("/proc/{pid}/task")))
}
