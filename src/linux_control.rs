use super::os_control::OsControl;
use anyhow::{Context, Result};
use std::io::Write;
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

        // Ensure the directory exists using sudo mkdir -p
        let mkdir_status = StdCommand::new("sudo")
            .arg("mkdir")
            .arg("-p")
            .arg(std::path::Path::new(OVERRIDE_PATH).parent().unwrap())
            .status()
            .context("Failed to execute `sudo mkdir -p`")?;
        if !mkdir_status.success() {
            anyhow::bail!("`sudo mkdir -p` failed with status {}", mkdir_status);
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
            // Use `sudo tee` to write the file content
            let mut child = StdCommand::new("sudo")
                .arg("tee")
                .arg(OVERRIDE_PATH)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::null()) // Don't care about tee's stdout
                .stderr(std::process::Stdio::piped())
                .spawn()
                .context("Failed to spawn `sudo tee` command for GDM banner")?;

            // Write content to the child's stdin
            if let Some(mut stdin) = child.stdin.take() {
                stdin
                    .write_all(content.as_bytes())
                    .context("Failed to write to `sudo tee` stdin for GDM banner")?;
            }

            // Wait for the command to finish and check for errors
            let output = child
                .wait_with_output()
                .context("`sudo tee` command failed for GDM banner")?;
            if !output.status.success() {
                anyhow::bail!(
                    "`sudo tee` failed with status {} for GDM banner: {}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        } else {
            // Deactivate banner by writing to the file
            let disable = "[org/gnome/login-screen]\nbanner-message-enable=false\n";
            let mut child = StdCommand::new("sudo")
                .arg("tee")
                .arg(OVERRIDE_PATH)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .context("Failed to spawn `sudo tee` command for GDM banner disable")?;

            if let Some(mut stdin) = child.stdin.take() {
                stdin
                    .write_all(disable.as_bytes())
                    .context("Failed to write to `sudo tee` stdin for GDM banner disable")?;
            }

            let output = child
                .wait_with_output()
                .context("`sudo tee` command failed for GDM banner disable")?;
            if !output.status.success() {
                anyhow::bail!(
                    "`sudo tee` failed with status {} for GDM banner disable: {}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr)
                );
            }
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
        use std::io::Write;
        const SCRIPT_PATH: &str = "/etc/profile.d/screamd-banner.sh";

        if let Some(msg) = message {
            // Escape single quotes for the shell, e.g. don't -> 'don'\''t'
            let escaped_msg = msg.replace('\'', "'\''");
            let content = format!("#!/bin/sh\necho '{}'", escaped_msg);

            // Use `sudo tee` to write the file content
            let mut child = StdCommand::new("sudo")
                .arg("tee")
                .arg(SCRIPT_PATH)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::null()) // Don't care about tee's stdout
                .stderr(std::process::Stdio::piped())
                .spawn()
                .context("Failed to spawn `sudo tee` command")?;

            // Write content to the child's stdin
            if let Some(mut stdin) = child.stdin.take() {
                stdin
                    .write_all(content.as_bytes())
                    .context("Failed to write to `sudo tee` stdin")?;
            }

            // Wait for the command to finish and check for errors
            let output = child
                .wait_with_output()
                .context("`sudo tee` command failed")?;
            if !output.status.success() {
                anyhow::bail!(
                    "`sudo tee` failed with status {}: {}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr)
                );
            }

            // Make the script executable
            let chmod_status = StdCommand::new("sudo")
                .arg("chmod")
                .arg("755")
                .arg(SCRIPT_PATH)
                .status()
                .context("Failed to execute `sudo chmod`")?;
            if !chmod_status.success() {
                anyhow::bail!("`sudo chmod` failed with status {}", chmod_status);
            }
        } else {
            // Remove the file using `sudo rm -f`
            let rm_status = StdCommand::new("sudo")
                .arg("rm")
                .arg("-f") // -f ignores "file not found" errors
                .arg(SCRIPT_PATH)
                .status()
                .context("Failed to execute `sudo rm`")?;
            if !rm_status.success() {
                anyhow::bail!("`sudo rm` failed with status {}", rm_status);
            }
        }
        Ok(())
    }
}