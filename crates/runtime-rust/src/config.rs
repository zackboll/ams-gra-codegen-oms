//! Stable runtime configuration, independent of `sleet_client::InitOptions`.

use crate::RuntimeError;
use std::time::Duration;

/// The OWP protocol version requested by default (OMSC-SPC-013 Rev B
/// describes "version 1.0 of the OMS WebSocket Protocol").
pub const DEFAULT_OWP_VERSION: &str = "1.0";

/// How long [`crate::SleetRuntime::connect`] waits for the WebSocket
/// upgrade plus `INIT`/`INFO` by default.
pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Finite upper bound on initial connection attempts, including the first.
pub const MAX_INITIAL_CONNECT_ATTEMPTS: u8 = 8;

/// Opt-in retry of refused TCP connections, never of an established session.
/// Construct through [`RuntimeConfig::with_initial_connect_retry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectRetryPolicy {
    pub(crate) max_attempts: u8,
    pub(crate) initial_delay: Duration,
    pub(crate) max_delay: Duration,
}

impl ConnectRetryPolicy {
    /// Total attempt budget (the first attempt is included).
    #[must_use]
    pub const fn max_attempts(&self) -> u8 {
        self.max_attempts
    }

    #[must_use]
    pub const fn initial_delay(&self) -> Duration {
        self.initial_delay
    }

    #[must_use]
    pub const fn max_delay(&self) -> Duration {
        self.max_delay
    }

    pub(crate) fn delay_after(&self, attempt: u8) -> Option<Duration> {
        if attempt == 0 || attempt >= self.max_attempts {
            return None;
        }
        let mut delay = self.initial_delay.min(self.max_delay);
        for _ in 1..attempt {
            delay = delay
                .checked_mul(2)
                .unwrap_or(self.max_delay)
                .min(self.max_delay);
        }
        Some(delay)
    }
}

impl Default for ConnectRetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 1,
            initial_delay: Duration::ZERO,
            max_delay: Duration::ZERO,
        }
    }
}

/// Connection settings for one LA-CAL client connection.
///
/// The schema version is always caller-supplied: no UCI version is assumed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeConfig {
    url: String,
    service_id: String,
    schema_version: String,
    owp_versions: Vec<String>,
    connect_timeout: Duration,
    initial_connect_retry: ConnectRetryPolicy,
}

impl RuntimeConfig {
    /// Settings for `url` (e.g. `ws://127.0.0.1:9000`), the OWP `service_id`
    /// sent in `INIT`, and the `schema` version sent in `INIT`.
    #[must_use]
    pub fn new(
        url: impl Into<String>,
        service_id: impl Into<String>,
        schema_version: impl Into<String>,
    ) -> Self {
        Self {
            url: url.into(),
            service_id: service_id.into(),
            schema_version: schema_version.into(),
            owp_versions: vec![DEFAULT_OWP_VERSION.to_owned()],
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            initial_connect_retry: ConnectRetryPolicy::default(),
        }
    }

    /// Replace the requested OWP protocol versions (default `["1.0"]`).
    #[must_use]
    pub fn with_owp_versions(mut self, versions: Vec<String>) -> Self {
        self.owp_versions = versions;
        self
    }

    /// Replace the connect/handshake timeout, applied separately per attempt.
    #[must_use]
    pub const fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Retry only initial TCP connection refusal, with deterministic exponential
    /// backoff. `max_attempts` includes the first and must be 1..=8. With retries,
    /// `initial_delay` must be positive; `max_delay` must be at least that delay.
    /// Invalid settings yield [`RuntimeError::InvalidConfig`] on connect.
    ///
    /// The campaign's wait budget is `max_attempts * connect_timeout` plus
    /// the retry delays, not one connect timeout. No retry/replay after INFO.
    #[must_use]
    pub const fn with_initial_connect_retry(
        mut self,
        max_attempts: u8,
        initial_delay: Duration,
        max_delay: Duration,
    ) -> Self {
        self.initial_connect_retry = ConnectRetryPolicy {
            max_attempts,
            initial_delay,
            max_delay,
        };
        self
    }

    #[must_use]
    pub const fn initial_connect_retry(&self) -> &ConnectRetryPolicy {
        &self.initial_connect_retry
    }

    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    #[must_use]
    pub fn service_id(&self) -> &str {
        &self.service_id
    }

    #[must_use]
    pub fn schema_version(&self) -> &str {
        &self.schema_version
    }

    #[must_use]
    pub fn owp_versions(&self) -> &[String] {
        &self.owp_versions
    }

    #[must_use]
    pub const fn connect_timeout(&self) -> Duration {
        self.connect_timeout
    }

    /// Reject configuration that cannot describe an `INIT`. Lexical OWP
    /// identifier rules (e.g. for `service_id`) stay with `sleet-client`.
    pub(crate) fn validate(&self) -> Result<(), RuntimeError> {
        let empty = |what: &str| Err(RuntimeError::InvalidConfig(format!("{what} is empty")));
        if self.url.is_empty() {
            return empty("url");
        }
        if self.service_id.is_empty() {
            return empty("service_id");
        }
        if self.schema_version.is_empty() {
            return empty("schema_version");
        }
        if self.owp_versions.is_empty() || self.owp_versions.iter().any(String::is_empty) {
            return empty("an OWP version");
        }
        if self.connect_timeout.is_zero() {
            return Err(RuntimeError::InvalidConfig(
                "connect_timeout is zero".to_owned(),
            ));
        }
        let policy = self.initial_connect_retry;
        if !(1..=MAX_INITIAL_CONNECT_ATTEMPTS).contains(&policy.max_attempts)
            || (policy.max_attempts > 1 && policy.initial_delay.is_zero())
            || policy.max_delay < policy.initial_delay
        {
            return Err(RuntimeError::InvalidConfig(
                "initial retry requires 1..=8 attempts, positive retry delay, and max_delay >= initial_delay".to_owned(),
            ));
        }
        // Tokio deadlines use Instant. Reject unrepresentable timer durations
        // rather than allowing a sleep to panic, even for a single attempt.
        let now = std::time::Instant::now();
        let mut budget = self
            .connect_timeout
            .checked_mul(u32::from(policy.max_attempts));
        for attempt in 1..policy.max_attempts {
            budget = budget.and_then(|budget| budget.checked_add(policy.delay_after(attempt)?));
        }
        if now.checked_add(self.connect_timeout).is_none()
            || now.checked_add(policy.initial_delay).is_none()
            || now.checked_add(policy.max_delay).is_none()
            || budget.and_then(|budget| now.checked_add(budget)).is_none()
        {
            return Err(RuntimeError::InvalidConfig(
                "timer duration is not representable".to_owned(),
            ));
        }
        Ok(())
    }

    /// The pinned client's INIT options. Non-verbose by design (see the
    /// crate documentation); size limits keep the pinned client defaults.
    pub(crate) fn init_options(&self) -> sleet_client::InitOptions {
        sleet_client::InitOptions {
            versions: self.owp_versions.clone(),
            schema: self.schema_version.clone(),
            verbose: false,
            ..sleet_client::InitOptions::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_init_options() {
        let config = RuntimeConfig::new("ws://h:1", "svc", "000.1.0");
        assert_eq!(config.owp_versions(), ["1.0".to_owned()]);
        config.validate().expect("valid");
        let init = config.init_options();
        assert_eq!(init.versions, ["1.0".to_owned()]);
        assert_eq!(init.schema, "000.1.0");
        assert!(!init.verbose);
    }

    #[test]
    fn empty_fields_are_configuration_errors() {
        for config in [
            RuntimeConfig::new("", "svc", "1"),
            RuntimeConfig::new("ws://h:1", "", "1"),
            RuntimeConfig::new("ws://h:1", "svc", ""),
            RuntimeConfig::new("ws://h:1", "svc", "1").with_owp_versions(Vec::new()),
            RuntimeConfig::new("ws://h:1", "svc", "1").with_connect_timeout(Duration::ZERO),
        ] {
            assert!(matches!(
                config.validate(),
                Err(RuntimeError::InvalidConfig(_))
            ));
        }
    }
}
