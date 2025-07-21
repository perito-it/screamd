use crate::os_control::OsControl;
use anyhow::{Context, Result};
use chrono::{Duration as ChronoDuration, NaiveTime, TimeZone, Utc};
use serde::Deserialize;
use std::fmt;
use std::fs;
use std::path::Path;
use std::sync::Arc;

#[derive(Deserialize, Clone)]
pub struct Config {
    pub debug: bool,
    pub warn_message: String,
    pub warn_duration_days: i64,
    pub reboot_duration_days: i64,
    pub warn_interval_seconds: u64,
    pub reboot_time: String,
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Debug: {}\n  Warn Message: {}\n  Warn Duration: {} days\n  Reboot Duration: {} days\n  Warn Interval: {} seconds\n  Reboot Time: {}",
            self.debug,
            self.warn_message,
            self.warn_duration_days,
            self.reboot_duration_days,
            self.warn_interval_seconds,
            self.reboot_time
        )
    }
}

struct State {
    start_time: chrono::DateTime<Utc>,
}

pub async fn run_service<C: OsControl>(
    os: C,
    config: Config,
    state_path: &Path,
) -> anyhow::Result<()> {
    println!(
        "Service started with configuration:
{config}"
    );
    let state = load_or_init_state(state_path)?;
    let now = Utc::now();

    let warn_duration = if config.debug {
        ChronoDuration::minutes(config.warn_duration_days)
    } else {
        ChronoDuration::days(config.warn_duration_days)
    };

    let reboot_duration = if config.debug {
        ChronoDuration::minutes(config.reboot_duration_days)
    } else {
        ChronoDuration::days(config.reboot_duration_days)
    };

    let warn_deadline = state.start_time + warn_duration;
    let reboot_deadline = warn_deadline + reboot_duration;

    // Wrap the OS abstraction in an Arc
    let os = Arc::new(os);

    // Set banner
    if now < warn_deadline {
        println!("Current status: Warning phase. Setting login banner.");
        os.set_login_banner(Some(&config.warn_message))?;
        os.set_shell_login_banner(Some(&config.warn_message))?;
    } else if now < reboot_deadline {
        println!("Current status: Reboot phase. Login banner will not be set.");
        os.set_login_banner(None)?;
        os.set_shell_login_banner(None)?;
    } else {
        println!("Current status: Shutdown phase. Login banner will not be set.");
        os.set_login_banner(None)?;
        os.set_shell_login_banner(None)?;
    }

    // Warning loop
    if now < warn_deadline {
        let os_clone = os.clone();
        let msg = config.warn_message.clone();
        let interval = std::time::Duration::from_secs(config.warn_interval_seconds);
        tokio::spawn(async move {
            loop {
                println!("Showing warning message.");
                os_clone.show_warning(&msg).await;
                println!("Sleeping for {} seconds.", interval.as_secs());
                tokio::time::sleep(interval).await;
            }
        });
    }

    // Reboot or shutdown loop
    if now < warn_deadline {
        std::future::pending::<()>().await;
    } else if now < reboot_deadline {
        let os_clone = os.clone();
        if config.debug {
            println!("Debug mode: Initiating immediate reboot as reboot period has ended.");
            os_clone.reboot().await?;
        } else {
            let reboot_time = NaiveTime::parse_from_str(&config.reboot_time, "%H:%M:%S")
                .or_else(|_| NaiveTime::parse_from_str(&config.reboot_time, "%H:%M"))
                .with_context(|| format!("Invalid reboot_time format: {}", config.reboot_time))?;

            loop {
                let now = Utc::now();
                let mut next_reboot = Utc
                    .from_local_datetime(&now.date_naive().and_time(reboot_time))
                    .unwrap();
                if now >= next_reboot {
                    next_reboot += ChronoDuration::days(1);
                }
                let sleep_duration = next_reboot - now;

                println!("Next reboot scheduled for: {next_reboot}");
                println!("Current time: {now}");
                println!("Sleep duration: {sleep_duration:?}");
                tokio::time::sleep(sleep_duration.to_std()?).await;

                println!("Initiating reboot.");
                os_clone.reboot().await?;
            }
        }
    } else {
        println!("Initiating shutdown.");
        os.shutdown().await?;
    }

    Ok(())
}

fn load_or_init_state(path: &Path) -> Result<State> {
    if let Ok(s) = fs::read_to_string(path) {
        let t: String = serde_json::from_str(&s)
            .with_context(|| format!("Failed to deserialize state from {}", path.display()))?;
        let dt = chrono::DateTime::parse_from_rfc3339(&t)
            .with_context(|| format!("Failed to parse timestamp from {}", path.display()))?
            .with_timezone(&Utc);
        Ok(State { start_time: dt })
    } else {
        let now = Utc::now();
        fs::write(path, serde_json::to_string(&now.to_rfc3339())?)
            .with_context(|| format!("Failed to write initial state to {}", path.display()))?;
        Ok(State { start_time: now })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration as ChronoDuration, Utc};
    use std::sync::{Arc, Mutex};
    use tokio::time::Duration as TokioDuration;

    // A simple call recorder for OsControl
    #[derive(Clone, Default)]
    struct MockOs {
        pub warnings: Arc<Mutex<u32>>,
        pub reboots: Arc<Mutex<u32>>,
        pub shutdowns: Arc<Mutex<u32>>,
        pub banner: Arc<Mutex<Option<String>>>,
        pub shell_banner: Arc<Mutex<Option<String>>>,
    }

    #[async_trait::async_trait]
    impl OsControl for MockOs {
        async fn show_warning(&self, _message: &str) {
            *self.warnings.lock().unwrap() += 1;
        }
        fn set_login_banner(&self, message: Option<&str>) -> anyhow::Result<()> {
            *self.banner.lock().unwrap() = message.map(|s| s.to_string());
            Ok(())
        }
        fn set_shell_login_banner(&self, message: Option<&str>) -> anyhow::Result<()> {
            *self.shell_banner.lock().unwrap() = message.map(|s| s.to_string());
            Ok(())
        }
        async fn reboot(&self) -> anyhow::Result<()> {
            *self.reboots.lock().unwrap() += 1;
            Ok(())
        }
        async fn shutdown(&self) -> anyhow::Result<()> {
            *self.shutdowns.lock().unwrap() += 1;
            Ok(())
        }
    }

    /// Helper function: Create state with a custom start time
    fn write_state(start: chrono::DateTime<Utc>, path: &std::path::Path) {
        let data = serde_json::to_string(&start.to_rfc3339()).unwrap();
        std::fs::write(path, data).unwrap();
    }

    #[tokio::test]
    async fn warning_phase_sets_banner_and_warns() {
        // Given a config with a 1-day warning and reboot duration
        let cfg = Config {
            debug: false,
            warn_message: "X".into(),
            warn_duration_days: 1,
            reboot_duration_days: 1,
            warn_interval_seconds: 1,
            reboot_time: "12:00".into(),
        };

        // And a state file indicating the service has just started
        let tmp = tempfile::tempdir().unwrap();
        let state_path = tmp.path().join("state.json");
        write_state(Utc::now(), &state_path);

        // When the service is run with a mock OS
        let os = MockOs::default();
        let os_clone = os.clone();
        let handle = tokio::spawn(async move {
            // run_service blocks in the warning loop
            run_service(os_clone, cfg, &state_path).await.unwrap();
        });

        // After a short delay
        tokio::time::sleep(TokioDuration::from_millis(10)).await;

        // Then the banner should be set and warnings should be issued
        assert_eq!(*os.banner.lock().unwrap(), Some("X".into()));
        assert!(*os.warnings.lock().unwrap() > 0);

        // Clean up
        handle.abort();
    }

    #[tokio::test]
    async fn reboot_phase_triggers_reboots() {
        let now = Utc::now();
        let reboot_time = now + ChronoDuration::seconds(1);
        let cfg = Config {
            debug: false,
            warn_message: "X".into(),
            warn_duration_days: 0, // Warning phase immediately over
            reboot_duration_days: 1,
            warn_interval_seconds: 1,
            reboot_time: reboot_time.format("%H:%M:%S").to_string(),
        };
        let tmp = tempfile::tempdir().unwrap();
        let state_path = tmp.path().join("state.json");
        // Start more than 1 day ago -> we are already in reboot phase
        write_state(Utc::now(), &state_path);

        let os = MockOs::default();
        let os_clone = os.clone();
        let handle = tokio::spawn(async move {
            run_service(os_clone, cfg, &state_path).await.unwrap();
        });

        tokio::time::sleep(TokioDuration::from_millis(1100)).await;
        assert!(*os.reboots.lock().unwrap() > 0);
        handle.abort();
    }

    #[tokio::test]
    async fn shutdown_after_reboot_phase() {
        let cfg = Config {
            debug: false,
            warn_message: "X".into(),
            warn_duration_days: 0,
            reboot_duration_days: 0, // Reboot-Phase immediately over
            warn_interval_seconds: 1,
            reboot_time: "12:00".into(),
        };
        let tmp = tempfile::tempdir().unwrap();
        let state_path = tmp.path().join("state.json");
        // Start 1 week ago -> immediate shutdown
        write_state(Utc::now() - ChronoDuration::days(7), &state_path);

        let os = MockOs::default();
        run_service(os.clone(), cfg, &state_path).await.unwrap();
        // Direct instead of loop: shutdown called once
        assert_eq!(*os.shutdowns.lock().unwrap(), 1);
    }
}
