// Live OS evidence, not another transport or a proxy. Only this process's
// established client sockets to this test's broker port are counted.
fn connections(url: &str) -> usize {
    connection_inodes(url).len()
}

fn connection_inodes(url: &str) -> BTreeSet<String> {
    let port: u16 = url.rsplit(':').next().unwrap().parse().unwrap();
    let sockets: BTreeSet<_> = std::fs::read_dir("/proc/self/fd")
        .unwrap()
        .filter_map(|fd| std::fs::read_link(fd.ok()?.path()).ok())
        .filter_map(|path| {
            path.to_str()?
                .strip_prefix("socket:[")?
                .strip_suffix(']')
                .map(str::to_owned)
        })
        .collect();
    std::fs::read_to_string("/proc/net/tcp")
        .unwrap()
        .lines()
        .skip(1)
        .filter(|line| {
            let fields: Vec<_> = line.split_whitespace().collect();
            fields[2].ends_with(&format!(":{port:04X}"))
                && fields[3] == "01"
                && sockets.contains(fields[9])
        })
        .map(|line| line.split_whitespace().nth(9).unwrap().to_owned())
        .collect()
}

fn workers() -> BTreeSet<PathBuf> {
    std::fs::read_dir("/proc/self/task")
        .unwrap()
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            std::fs::read_to_string(path.join("comm"))
                .ok()?
                .starts_with("ams-gra-oms-lac")
                .then_some(path)
        })
        .collect()
}
