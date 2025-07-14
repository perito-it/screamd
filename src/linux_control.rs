use super::os_control::OsControl;
use anyhow::{Context, Result};
use std::fs;
use std::process::Command as StdCommand;
use tokio::process::Command;

pub struct LinuxControl {}

impl LinuxControl {
    pub fn new() -> Self {
        // The interval will be passed from the config later, this is a default for now
        Self {}
    }
}

#[async_trait::async_trait]
impl OsControl for LinuxControl {
    async fn show_warning(&self, message: &str) {
        let _ = Command::new("wall")
            .arg(message)
            .status()
            .await
            .context("Failed to execute `wall` command");
    }

    /// Sets or removes the GNOME/GDM login banner
    fn set_login_banner(&self, message: Option<&str>) -> Result<()> {
        // Path to the dconf override file for GDM
        const OVERRIDE_PATH: &str = "/etc/dconf/db/gdm.d/01-warn-message";

        // Ensure the directory exists
        if let Some(parent) = std::path::Path::new(OVERRIDE_PATH).parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create directory `{:?}`. Try running with sudo.",
                    parent
                )
            })?;
        }

        if let Some(msg) = message {
            // Activate banner and set text
            let content = format!(
                r#"[org/gnome/login-screen]
banner-message-enable=true
banner-message-text='{}'
"#,
                msg.replace('\'', "'")
            );
            fs::write(OVERRIDE_PATH, content).with_context(|| {
                format!(
                    "Failed to write to `{}`. Try running with sudo.",
                    OVERRIDE_PATH
                )
            })?;
        } else {
            // Deactivate banner
            let disable = "[org/gnome/login-screen]\nbanner-message-enable=false\n";
            fs::write(OVERRIDE_PATH, disable).with_context(|| {
                format!(
                    "Failed to write to `{}`. Try running with sudo.",
                    OVERRIDE_PATH
                )
            })?;
        }

        // Rebuild the dconf database so that GDM picks up the new banner
        let status = StdCommand::new("dconf")
            .arg("update")
            .status()
            .context("Failed to execute `dconf update`. Is `dconf-cli` installed?")?;
        if !status.success() {
            anyhow::bail!("`dconf update` failed with status {}", status);
        }

        Ok(())
    }

    async fn reboot(&self) -> Result<()> {
        let _ = Command::new("systemctl")
            .arg("reboot")
            .status()
            .await
            .context("Failed to execute `systemctl reboot`")?;
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        let _ = Command::new("systemctl")
            .arg("poweroff")
            .status()
            .await
            .context("Failed to execute `systemctl poweroff`")?;
        Ok(())
    }
}
