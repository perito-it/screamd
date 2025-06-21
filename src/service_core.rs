use crate::os_control::OsControl;
use anyhow::Result;
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

    // pack OS-Abstraktion in Arc
    let os = Arc::new(os);

    // Banner setzen
    if now < warn_deadline {
        os.set_login_banner(Some(&config.warn_message))?;
    } else {
        os.set_login_banner(None)?;
    }

    // Warn-Loop
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

    // Reboot- bzw. Shutdown-Loop
    if now < warn_deadline {
        std::future::pending::<()>().await;
    } else if now < reboot_deadline {
        let os_clone = os.clone();
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(24 * 3600)).await;
            os_clone.reboot().await?;
        }
    } else {
        os.shutdown().await?;
    }

    Ok(())
}

fn load_or_init_state() -> Result<State> {
    let path = "state.json";
    if let Ok(s) = fs::read_to_string(path) {
        let t: String = serde_json::from_str(&s)?;
        let dt = chrono::DateTime::parse_from_rfc3339(&t)?.with_timezone(&Utc);
        Ok(State { start_time: dt })
    } else {
        let now = Utc::now();
        fs::write(path, serde_json::to_string(&now.to_rfc3339())?)?;
        Ok(State { start_time: now })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use chrono::{Utc, Duration as ChronoDuration};
    use tokio::time::Duration as TokioDuration;

    // Ein einfacher Call-Recorder für OsControl
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
        fn warn_interval(&self) -> TokioDuration {
            TokioDuration::from_millis(1) // super-kurz, damit der Loop schnell hochzählt
        }
    }

    /// Hilfsfunktion: State mit individuellem Startzeitpunkt erzeugen
    fn write_state(start: chrono::DateTime<Utc>, path: &std::path::Path) {
        let data = serde_json::to_string(&start.to_rfc3339()).unwrap();
        std::fs::write(path, data).unwrap();
    }

    #[tokio::test]
    async fn warning_phase_sets_banner_and_warns() {
        // 1) Config: Warndauer 1 Tag, Reboot 1 Tag
        let cfg = Config {
            warn_message: "X".into(),
            warn_duration_days: 1,
            reboot_duration_days: 1,
            warn_interval_seconds: 1,
        };

        // 2) State: gerade erst gestartet
        let tmp = tempfile::tempdir().unwrap();
        let state_path = tmp.path().join("state.json");
        write_state(Utc::now(), &state_path);

        // 3) Temporäres Inject: override load_or_init_state, z.B. via ENV-Var
        std::env::set_var("STATE_PATH", &state_path);

        // 4) Mock und Service starten (in Tokio-Task, aber Timeout setzen)
        let os = MockOs::default();
        let os_clone = os.clone();
        let handle = tokio::spawn(async move {
            // run_service blockiert im Warn-Loop
            run_service(os_clone, cfg).await.unwrap();
        });

        // kurz warten
        tokio::time::sleep(TokioDuration::from_millis(10)).await;

        // 5) Assertions
        assert_eq!(*os.banner.lock().unwrap(), Some("X".into()));
        assert!(*os.warnings.lock().unwrap() > 0);

        // Aufräumen
        handle.abort();
    }

    #[tokio::test]
    async fn reboot_phase_triggers_reboots() {
        let cfg = Config {
            warn_message: "X".into(),
            warn_duration_days: 0,    // Warnphase sofort vorbei
            reboot_duration_days: 1,
            warn_interval_seconds: 1,
        };
        let tmp = tempfile::tempdir().unwrap();
        let state_path = tmp.path().join("state.json");
        // Start vor mehr als 1 Tag → wir sind schon in Reboot-Phase
        write_state(Utc::now() - ChronoDuration::days(2), &state_path);
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
            reboot_duration_days: 0,  // Reboot-Phase sofort vorbei
            warn_interval_seconds: 1,
        };
        let tmp = tempfile::tempdir().unwrap();
        let state_path = tmp.path().join("state.json");
        // Start vor 1 Woche → direkt Shutdown
        write_state(Utc::now() - ChronoDuration::days(7), &state_path);
        std::env::set_var("STATE_PATH", &state_path);

        let os = MockOs::default();
        run_service(os.clone(), cfg).await.unwrap();
        // Direkt statt Loop: shutdown einmal aufgerufen
        assert_eq!(*os.shutdowns.lock().unwrap(), 1);
    }
}

