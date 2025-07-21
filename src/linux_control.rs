use super::os_control::OsControl;
use anyhow::{Context, Result};
use std::process::Command as StdCommand;
use tokio::process::Command;

/// Provides Linux-specific OS control functionalities.
pub struct LinuxControl {}

impl LinuxControl {
    /// Creates a new instance of `LinuxControl`.
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl OsControl for LinuxControl {
    /// Displays a warning message to all logged-in users using `wall`.
    async fn show_warning(&self, message: &str) {
        let _ = Command::new("wall").arg(message).status().await;
    }

    /// Sets or removes the GNOME/GDM login banner by writing a dconf override file.
    /// This requires `dconf-cli` to be available and the user to have the correct permissions.
    fn set_login_banner(&self, message: Option<&str>) -> Result<()> {
        const OVERRIDE_PATH: &str = "/etc/dconf/db/gdm.d/01-warn-message";

        if let Some(msg) = message {
            // Escape characters for the dconf file format.
            let escaped_msg = msg.replace('\\', "\\").replace('\'', "\'");
            let content = format!(
                r#"[org/gnome/login-screen]
banner-message-enable=true
banner-message-text='{}'
"#,
                escaped_msg
            );
            std::fs::write(OVERRIDE_PATH, content)?;
        } else {
            // To disable the banner, we set banner-message-enable to false.
            let disable = "[org/gnome/login-screen]
banner-message-enable=false
";
            std::fs::write(OVERRIDE_PATH, disable)?;
        }

        // After changing dconf files, the dconf database needs to be updated.
        // This is typically done by running `dconf update`.
        // Note: This command needs to be run as root.
        let status = StdCommand::new("dconf")
            .arg("update")
            .status()
            .context("Failed to execute `dconf update`. Make sure dconf-cli is installed and you have permissions.")?;

        if !status.success() {
            eprintln!(
                "`dconf update` failed with status: {}. Login banner may not be updated.",
                status
            );
        }

        Ok(())
    }

    /// Reboots the system using `systemctl`.
    async fn reboot(&self) -> Result<()> {
        let _ = Command::new("systemctl")
            .arg("reboot")
            .status()
            .await
            .context("Failed to execute `systemctl reboot`")?;
        Ok(())
    }

    /// Shuts down the system using `systemctl`.
    async fn shutdown(&self) -> Result<()> {
        let _ = Command::new("systemctl")
            .arg("poweroff")
            .status()
            .await
            .context("Failed to execute `systemctl poweroff`")?;
        Ok(())
    }

    /// Sets or removes a banner for interactive shell logins.
    /// This is done by creating or deleting a script in `/etc/profile.d/`.
    fn set_shell_login_banner(&self, message: Option<&str>) -> Result<()> {
        const SCRIPT_PATH: &str = "/etc/profile.d/screamd-banner.sh";

        if let Some(msg) = message {
            // Escape single quotes for the shell script.
            let escaped_msg = msg.replace('\'', "'\''");
            let content = format!("#!/bin/sh
echo '{}'", escaped_msg);
            std::fs::write(SCRIPT_PATH, content)?;
        } else {
            // If no message is provided, remove the banner script.
            let _ = std::fs::remove_file(SCRIPT_PATH);
        }
        Ok(())
    }
}