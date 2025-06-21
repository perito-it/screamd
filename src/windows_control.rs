use super::os_control::OsControl;
use anyhow::Result;
use tokio::process::Command;
use tokio::time::Duration;
use winreg::enums::*;
use winreg::RegKey;

pub struct WindowsControl {
    interval: Duration,
}

impl WindowsControl {
    pub fn new() -> Result<Self> {
        Ok(Self { interval: Duration::from_secs(3600) })
    }
}

#[async_trait::async_trait]
impl OsControl for WindowsControl {
    async fn show_warning(&self, message: &str) {
        let _ = Command::new("msg").args(&["*", message]).status().await;
    }

    fn set_login_banner(&self, message: Option<&str>) -> Result<()> {
        // Registry-Schreibzugriff wie zuvor beschrieben
        // ...
        Ok(())
    }

    async fn reboot(&self) -> Result<()> {
        let _ = Command::new("shutdown").args(&["/r", "/t", "0"]).status().await;
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        let _ = Command::new("shutdown").args(&["/s", "/t", "0"]).status().await;
        Ok(())
    }

    fn warn_interval(&self) -> Duration {
        self.interval
    }
}
