use super::os_control::OsControl;
use anyhow::Result;
use tokio::process::Command;
use std::process::Command as StdCommand;
use tokio::time::Duration;
use std::fs;

pub struct LinuxControl {
    interval: Duration,
}

impl LinuxControl {
    pub fn new() -> Self {
        // Interval später aus Config übergeben, hier Default
        Self { interval: Duration::from_secs(3600) }
    }
}

#[async_trait::async_trait]
impl OsControl for LinuxControl {
    async fn show_warning(&self, message: &str) {
        let _ = Command::new("wall")
            .arg(message)
            .status()
            .await;
    }


    /// Setzt oder entfernt das GNOME/GDM Login-Banner
    fn set_login_banner(&self, message: Option<&str>) -> Result<()> {
        // Pfad zur dconf-Override-Datei für GDM
        const OVERRIDE_PATH: &str = "/etc/dconf/db/gdm.d/01-warn-message";

        if let Some(msg) = message {
            // Banner aktivieren und Text setzen
            let content = format!(
r#"[org/gnome/login-screen]
banner-message-enable=true
banner-message-text='{}'
"#,
                msg.replace('\'', "\\'")
            );
            fs::write(OVERRIDE_PATH, content)?;
        } else {
            // Banner deaktivieren
            let disable = "[org/gnome/login-screen]\nbanner-message-enable=false\n";
            fs::write(OVERRIDE_PATH, disable)?;
        }

        // dconf-Datenbank neu aufbauen, damit GDM das neue Banner übernimmt
        let status = StdCommand::new("dconf")
            .arg("update")
            .status()?;
        if !status.success() {
            anyhow::bail!("dconf update failed with status {}", status);
        }

        Ok(())
    }


    async fn reboot(&self) -> Result<()> {
        let _ = Command::new("systemctl").arg("reboot").status().await;
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        let _ = Command::new("systemctl").arg("poweroff").status().await;
        Ok(())
    }

    fn warn_interval(&self) -> Duration {
        self.interval
    }
}
