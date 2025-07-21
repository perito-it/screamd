use super::os_control::OsControl;
use anyhow::Result;
use tokio::process::Command;

use winreg::enums::*;
use winreg::RegKey;

pub struct WindowsControl {}

impl WindowsControl {
    pub fn new() -> Result<Self> {
        Ok(Self {})
    }
}

#[async_trait::async_trait]
impl OsControl for WindowsControl {
    async fn show_warning(&self, message: &str) {
        let _ = Command::new("msg").args(&["*", message]).status().await;
    }

    fn set_login_banner(&self, message: Option<&str>) -> Result<()> {
        // Registry access as described before
        // ...
        Ok(())
    }

    async fn reboot(&self) -> Result<()> {
        let _ = Command::new("shutdown")
            .args(&["/r", "/t", "0"])
            .status()
            .await;
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        let _ = Command::new("shutdown")
            .args(&["/s", "/t", "0"])
            .status()
            .await;
        Ok(())
    }
}
