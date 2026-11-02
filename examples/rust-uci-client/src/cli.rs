use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Duration;

pub const HELP: &str = "rust-uci-client --url ws://HOST:PORT --service-id ID [OPTIONS]

Options:
  --url URL                 Sleet WebSocket endpoint (ws://, no TLS)
  --service-id ID           OWP service identity authorized by Sleet
  --duration-seconds N      Receive for 1..=3600 seconds (default: 5)
  --reconnect-attempts N    Process-wide additional attempts, 0..=8 (default: 0)
  --reconnect-delay-ms N    Delay between attempts, 50..=2000 (default: 250)
  --publish-log FILE        OMS JSON Log payload BODY (no global envelope)
  --publish-rdma FILE       OMS JSON RDMA_InitializeSetup payload BODY
  --publish-survivability FILE  OMS JSON SurvivabilityRiskLevel payload BODY
  --publish-dlz FILE        OMS JSON DLZ payload BODY
  --help                    Print this usage without connecting

Build with AMS_GRA_UCI_ROOT and AMS_GRA_UCI_RELEASE=2.5 or 2.6.
The compiled release fixes INIT's schema version; there is no runtime override.
Publication success means frame written, NOT broker acceptance.
";

#[derive(Debug)]
pub struct Options {
    pub url: String,
    pub service_id: String,
    pub duration: Duration,
    pub reconnect_attempts: u8,
    pub reconnect_delay: Duration,
    pub log: Option<PathBuf>,
    pub rdma: Option<PathBuf>,
    pub survivability: Option<PathBuf>,
    pub dlz: Option<PathBuf>,
}

impl Options {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Option<Self>, String> {
        let args: Vec<_> = args.into_iter().collect();
        if args == ["--help"] {
            return Ok(None);
        }
        let mut options = Self {
            url: String::new(),
            service_id: String::new(),
            duration: Duration::from_secs(5),
            reconnect_attempts: 0,
            reconnect_delay: Duration::from_millis(250),
            log: None,
            rdma: None,
            survivability: None,
            dlz: None,
        };
        let mut seen = BTreeSet::new();
        let mut args = args.into_iter();
        while let Some(flag) = args.next() {
            if !matches!(
                flag.as_str(),
                "--url"
                    | "--service-id"
                    | "--duration-seconds"
                    | "--reconnect-attempts"
                    | "--reconnect-delay-ms"
                    | "--publish-log"
                    | "--publish-rdma"
                    | "--publish-survivability"
                    | "--publish-dlz"
            ) {
                return Err(format!("unknown argument {flag:?}; use --help"));
            }
            if !seen.insert(flag.clone()) {
                return Err(format!("duplicate argument {flag}"));
            }
            let value = args
                .next()
                .filter(|value| !value.is_empty() && !value.starts_with("--"))
                .ok_or_else(|| format!("{flag} requires a value; use --help"))?;
            match flag.as_str() {
                "--url" => options.url = value,
                "--service-id" => options.service_id = value,
                "--publish-log" => options.log = Some(value.into()),
                "--publish-rdma" => options.rdma = Some(value.into()),
                "--publish-survivability" => options.survivability = Some(value.into()),
                "--publish-dlz" => options.dlz = Some(value.into()),
                "--reconnect-attempts" => {
                    options.reconnect_attempts = value
                        .parse::<u8>()
                        .ok()
                        .filter(|n| *n <= 8)
                        .ok_or("--reconnect-attempts must be an integer in 0..=8")?;
                }
                "--reconnect-delay-ms" => {
                    let millis = value
                        .parse::<u64>()
                        .ok()
                        .filter(|n| (50..=2000).contains(n))
                        .ok_or("--reconnect-delay-ms must be an integer in 50..=2000")?;
                    options.reconnect_delay = Duration::from_millis(millis);
                }
                "--duration-seconds" => {
                    let seconds = value
                        .parse::<u64>()
                        .ok()
                        .filter(|n| (1..=3600).contains(n))
                        .ok_or("--duration-seconds must be an integer in 1..=3600")?;
                    options.duration = Duration::from_secs(seconds);
                }
                _ => unreachable!("known flag"),
            }
        }
        let authority = options
            .url
            .strip_prefix("ws://")
            .and_then(|rest| rest.split(['/', '?', '#']).next())
            .unwrap_or("");
        if authority.is_empty()
            || authority.contains('@')
            || options.url.chars().any(char::is_whitespace)
            || options.url.contains('#')
        {
            return Err("--url must be a ws://HOST[:PORT][/PATH] endpoint without credentials, whitespace or fragment".into());
        }
        if options.service_id.is_empty()
            || !options
                .service_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
        {
            return Err(
                "--service-id must be a nonempty OWP identifier: ASCII letters, digits, _, - or ."
                    .into(),
            );
        }
        Ok(Some(options))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(tail: &[&str]) -> Result<Option<Options>, String> {
        Options::parse(
            [
                "--url",
                "ws://127.0.0.1:9000",
                "--service-id",
                "rust-uci-client",
            ]
            .into_iter()
            .chain(tail.iter().copied())
            .map(str::to_owned),
        )
    }

    #[test]
    fn help_and_valid_configuration() {
        assert!(Options::parse(["--help".into()]).unwrap().is_none());
        let options = parse(&[
            "--duration-seconds",
            "1",
            "--publish-log",
            "log.json",
            "--publish-rdma",
            "rdma.json",
            "--publish-survivability",
            "survivability.json",
            "--publish-dlz",
            "dlz.json",
        ])
        .unwrap()
        .unwrap();
        assert_eq!(options.duration, Duration::from_secs(1));
        assert_eq!(options.log, Some("log.json".into()));
        assert_eq!(options.rdma, Some("rdma.json".into()));
        assert_eq!(options.survivability, Some("survivability.json".into()));
        assert_eq!(options.dlz, Some("dlz.json".into()));
        assert_eq!(
            parse(&[]).unwrap().unwrap().duration,
            Duration::from_secs(5)
        );
    }

    #[test]
    fn bounded_reconnect_configuration() {
        let defaults = parse(&[]).unwrap().unwrap();
        assert_eq!(defaults.reconnect_attempts, 0);
        assert_eq!(defaults.reconnect_delay, Duration::from_millis(250));
        for attempts in ["0", "8"] {
            for delay in ["50", "2000"] {
                let options = parse(&[
                    "--reconnect-attempts",
                    attempts,
                    "--reconnect-delay-ms",
                    delay,
                ])
                .unwrap()
                .unwrap();
                assert_eq!(options.reconnect_attempts, attempts.parse::<u8>().unwrap());
                assert_eq!(
                    options.reconnect_delay,
                    Duration::from_millis(delay.parse().unwrap())
                );
            }
        }
        for (flag, invalid) in [
            (
                "--reconnect-attempts",
                vec!["-1", "9", "256", "1.5", "oops", ""],
            ),
            (
                "--reconnect-delay-ms",
                vec!["-1", "0", "49", "2001", "1.5", "oops", ""],
            ),
        ] {
            assert!(parse(&[flag]).is_err());
            for value in invalid {
                assert!(parse(&[flag, value]).is_err());
            }
            assert!(parse(&[flag, "1", flag, "1"]).is_err());
        }
    }

    #[test]
    fn rejects_bad_arguments_before_connect() {
        for args in [
            vec!["--wat"],
            vec!["--publish-log"],
            vec!["--publish-rdma", "--help"],
            vec!["--publish-survivability"],
            vec!["--publish-dlz", "--help"],
            vec!["--publish-dlz", "a.json", "--publish-dlz", "b.json"],
            vec![
                "--publish-survivability",
                "a.json",
                "--publish-survivability",
                "b.json",
            ],
            vec!["--duration-seconds", "0"],
            vec!["--duration-seconds", "3601"],
            vec!["--duration-seconds", "-1"],
            vec!["--duration-seconds", "oops"],
            vec!["--url", "ws://elsewhere"],
        ] {
            assert!(parse(&args).is_err(), "{args:?}");
        }
        for (url, id) in [
            ("http://localhost", "ok"),
            ("ws://", "ok"),
            ("ws://user@host", "ok"),
            ("ws://host/#x", "ok"),
            ("ws://host", "bad id"),
            ("ws://host", "非ASCII"),
        ] {
            assert!(Options::parse(["--url", url, "--service-id", id].map(str::to_owned)).is_err());
        }
        assert!(Options::parse(Vec::new()).is_err());
    }
}
