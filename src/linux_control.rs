use super::os_control::OsControl;
use anyhow::{Context, Result};
use std::fs;
use std::process::Command as StdCommand;
use tokio::io::AsyncWriteExt;
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
        let mut child = match Command::new("sudo")
            .arg("wall")
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(e) => {
                eprintln!("Failed to execute `wall` command: {}", e);
                return;
            }
        };

        if let Some(mut stdin) = child.stdin.take() {
            if let Err(e) = stdin.write_all(message.as_bytes()).await {
                eprintln!("Failed to write to `wall` stdin: {}", e);
            }
        }

        if let Err(e) = child.wait().await {
            eprintln!("`wall` command failed: {}", e);
        }
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
            // Activate banner and set text, escaping characters for dconf
            let escaped_msg = msg.replace('\\', "\\").replace('\'', "\'");
            let content = format!(
                r#"[org/gnome/login-screen]
banner-message-enable=true
banner-message-text='{}'
"#,
                escaped_msg
            );
            fs::write(OVERRIDE_PATH, content).with_context(|| {
                format!(
                    "Failed to write to `{}`. Try running with sudo.",
                    OVERRIDE_PATH
                )
            })?;
        } else {
            // Deactivate banner
            let disable = "[org/gnome/login-screen]
banner-message-enable=false
";
            fs::write(OVERRIDE_PATH, disable).with_context(|| {
                format!(
                    "Failed to write to `{}`. Try running with sudo.",
                    OVERRIDE_PATH
                )
            })?;
        }

        // Rebuild the dconf database so that GDM picks up the new banner
        let status = StdCommand::new("sudo")
            .arg("dconf")
            .arg("update")
            .status()
            .context("Failed to execute `dconf update`. Is `dconf-cli` installed?")?;
        if !status.success() {
            anyhow::bail!("`dconf update` failed with status {}", status);
        }

        Ok(())
    }

    async fn reboot(&self) -> Result<()> {
        let _ = Command::new("sudo")
            .arg("systemctl")
            .arg("reboot")
            .status()
            .await
            .context("Failed to execute `systemctl reboot`")?;
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        let _ = Command::new("sudo")
            .arg("systemctl")
            .arg("poweroff")
            .status()
            .await
            .context("Failed to execute `systemctl poweroff`")?;
        Ok(())
    }

    fn set_shell_login_banner(&self, message: Option<&str>) -> Result<()> {
        const SCRIPT_PATH: &str = "/etc/profile.d/screamd-banner.sh";
        if let Some(msg) = message {
            // Escape single quotes for the shell
            let escaped_msg = msg.replace('\'', "'\''");
            let content = format!("#!/bin/sh\necho '{}'", escaped_msg);
            fs::write(SCRIPT_PATH, content)?;
            // Make the script executable
            let perms = std::os::unix::fs::PermissionsExt::from_mode(0o755);
            fs::set_permissions(SCRIPT_PATH, perms)?;
        } else {
            // Remove the file if it exists
            if std::path::Path::new(SCRIPT_PATH).exists() {
                fs::remove_file(SCRIPT_PATH)?;
            }
        }
        Ok(())
    }
}
