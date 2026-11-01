use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Duration;

pub const HELP: &str = "rust-uci-client --url ws://HOST:PORT --service-id ID [OPTIONS]

Options:
  --url URL                 Sleet WebSocket endpoint (ws://, no TLS)
  --service-id ID           OWP service identity authorized by Sleet
  --duration-seconds N      Receive for 1..=3600 seconds (default: 5)
  --publish-log FILE        OMS JSON Log payload BODY (no global envelope)
  --publish-rdma FILE       OMS JSON RDMA_InitializeSetup payload BODY
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
    pub log: Option<PathBuf>,
    pub rdma: Option<PathBuf>,
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
            log: None,
            rdma: None,
        };
        let mut seen = BTreeSet::new();
        let mut args = args.into_iter();
        while let Some(flag) = args.next() {
            if !matches!(
                flag.as_str(),
                "--url"
                    | "--service-id"
                    | "--duration-seconds"
                    | "--publish-log"
                    | "--publish-rdma"
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
        ])
        .unwrap()
        .unwrap();
        assert_eq!(options.duration, Duration::from_secs(1));
        assert_eq!(options.log, Some("log.json".into()));
        assert_eq!(options.rdma, Some("rdma.json".into()));
        assert_eq!(
            parse(&[]).unwrap().unwrap().duration,
            Duration::from_secs(5)
        );
    }

    #[test]
    fn rejects_bad_arguments_before_connect() {
        for args in [
            vec!["--wat"],
            vec!["--publish-log"],
            vec!["--publish-rdma", "--help"],
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
