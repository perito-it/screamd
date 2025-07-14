use crate::os_control::OsControl;
use anyhow::{Context, Result};
use chrono::{Duration as ChronoDuration, Utc};
use serde::Deserialize;
use std::fs;
use std::sync::Arc;

#[derive(Deserialize, Clone)]
pub struct Config {
    pub warn_message: String,
    pub warn_duration_days: i64,
    pub reboot_duration_days: i64,
    pub warn_interval_seconds: u64,
}

struct State {
    start_time: chrono::DateTime<Utc>,
}

pub async fn run_service<C: OsControl>(
    os: C,
    config: Config,
) -> anyhow::Result<()> {
    let state = load_or_init_state()?;
    let now = Utc::now();
    let warn_deadline = state.start_time + ChronoDuration::days(config.warn_duration_days);
    let reboot_deadline = warn_deadline + ChronoDuration::days(config.reboot_duration_days);

    // Wrap the OS abstraction in an Arc
    let os = Arc::new(os);

    // Set banner
    if now < warn_deadline {
        os.set_login_banner(Some(&config.warn_message))?;
    } else {
        os.set_login_banner(None)?;
    }

    // Warning loop
    if now < warn_deadline {
        let os_clone = os.clone();
        let msg = config.warn_message.clone();
        let interval = std::time::Duration::from_secs(config.warn_interval_seconds);
        tokio::spawn(async move {
            loop {
                os_clone.show_warning(&msg).await;
                tokio::time::sleep(interval).await;
            }
        });
    }

    // Reboot or shutdown loop
    if now < warn_deadline {
        std::future::pending::<()>().await;
    } else if now < reboot_deadline {
        let os_clone = os.clone();
        loop {
            os_clone.reboot().await?;
            tokio::time::sleep(tokio::time::Duration::from_secs(24 * 3600)).await;
        }
    } else {
        os.shutdown().await?;
    }

    Ok(())
}

fn load_or_init_state() -> Result<State> {
    // Try to get path from env var, otherwise build it from exe path
    let path = match std::env::var("STATE_PATH") {
        Ok(p) => std::path::PathBuf::from(p),
        Err(_) => {
            let exe_path = std::env::current_exe().context("Failed to get current executable path")?;
            let exe_dir = exe_path
                .parent()
                .context("Failed to get parent directory of executable")?;
            exe_dir.join("state.json")
        }
    };

    if let Ok(s) = fs::read_to_string(&path) {
        let t: String = serde_json::from_str(&s)
            .with_context(|| format!("Failed to deserialize state from {}", path.display()))?;
        let dt = chrono::DateTime::parse_from_rfc3339(&t)
            .with_context(|| format!("Failed to parse timestamp from {}", path.display()))?
            .with_timezone(&Utc);
        Ok(State { start_time: dt })
    } else {
        let now = Utc::now();
        fs::write(&path, serde_json::to_string(&now.to_rfc3339())?)
            .with_context(|| format!("Failed to write initial state to {}", path.display()))?;
        Ok(State { start_time: now })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use chrono::{Utc, Duration as ChronoDuration};
    use tokio::time::Duration as TokioDuration;

    // A simple call recorder for OsControl
    #[derive(Clone, Default)]
    struct MockOs {
        pub warnings: Arc<Mutex<u32>>,
        pub reboots: Arc<Mutex<u32>>,
        pub shutdowns: Arc<Mutex<u32>>,
        pub banner: Arc<Mutex<Option<String>>>,
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
        // 1) Config: Warn duration 1 day, reboot 1 day
        let cfg = Config {
            warn_message: "X".into(),
            warn_duration_days: 1,
            reboot_duration_days: 1,
            warn_interval_seconds: 1,
        };

        // 2) State: just started
        let tmp = tempfile::tempdir().unwrap();
        let state_path = tmp.path().join("state.json");
        write_state(Utc::now(), &state_path);

        // 3) Temporary inject: override load_or_init_state, e.g. via ENV var
        std::env::set_var("STATE_PATH", &state_path);

        // 4) Start mock and service (in Tokio task, but set a timeout)
        let os = MockOs::default();
        let os_clone = os.clone();
        let handle = tokio::spawn(async move {
            // run_service blocks in the warning loop
            run_service(os_clone, cfg).await.unwrap();
        });

        // wait a bit
        tokio::time::sleep(TokioDuration::from_millis(10)).await;

        // 5) Assertions
        assert_eq!(*os.banner.lock().unwrap(), Some("X".into()));
        assert!(*os.warnings.lock().unwrap() > 0);

        // Clean up
        handle.abort();
    }

        #[tokio::test]
    async fn reboot_phase_triggers_reboots() {
        let cfg = Config {
            warn_message: "X".into(),
            warn_duration_days: 0,    // Warning phase immediately over
            reboot_duration_days: 1,
            warn_interval_seconds: 1,
        };
        let tmp = tempfile::tempdir().unwrap();
        let state_path = tmp.path().join("state.json");
        // Start more than 1 day ago -> we are already in reboot phase
        write_state(Utc::now(), &state_path);
        std::env::set_var("STATE_PATH", &state_path);

        let os = MockOs::default();
        let os_clone = os.clone();
        let handle = tokio::spawn(async move {
            run_service(os_clone, cfg).await.unwrap();
        });

        tokio::time::sleep(TokioDuration::from_millis(10)).await;
        assert!(*os.reboots.lock().unwrap() > 0);
        handle.abort();
    }

    #[tokio::test]
    async fn shutdown_after_reboot_phase() {
        let cfg = Config {
            warn_message: "X".into(),
            warn_duration_days: 0,
            reboot_duration_days: 0,  // Reboot-Phase immediately over
            warn_interval_seconds: 1,
        };
        let tmp = tempfile::tempdir().unwrap();
        let state_path = tmp.path().join("state.json");
        // Start 1 week ago -> immediate shutdown
        write_state(Utc::now() - ChronoDuration::days(7), &state_path);
        std::env::set_var("STATE_PATH", &state_path);

        let os = MockOs::default();
        run_service(os.clone(), cfg).await.unwrap();
        // Direct instead of loop: shutdown called once
        assert_eq!(*os.shutdowns.lock().unwrap(), 1);
    }
}

