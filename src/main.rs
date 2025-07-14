mod os_control;
mod service_core;
use anyhow::{Context, Result};

#[cfg(target_os = "linux")]
mod linux_control;
#[cfg(target_os = "windows")]
mod windows_control;

use service_core::run_service;

fn load_config() -> Result<service_core::Config> {
    let exe_path = std::env::current_exe().context("Failed to get current executable path")?;
    let exe_dir = exe_path
        .parent()
        .context("Failed to get parent directory of executable")?;
    let config_path = exe_dir.join("config.toml");

    let config_str = std::fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file at `{}`", config_path.display()))?;

    let config: service_core::Config = toml::from_str(&config_str)
        .with_context(|| format!("Failed to parse TOML in `{}`", config_path.display()))?;

    if config.warn_duration_days < 0 {
        anyhow::bail!(
            "Invalid configuration: warn_duration_days ({}) must be >= 0",
            config.warn_duration_days
        );
    }
    if config.reboot_duration_days < 0 {
        anyhow::bail!(
            "Invalid configuration: reboot_duration_days ({}) must be >= 0",
            config.reboot_duration_days
        );
    }
    if config.warn_interval_seconds == 0 {
        anyhow::bail!("Invalid configuration: warn_interval_seconds must be > 0");
    }

    Ok(config)
}


#[tokio::main]
async fn main() -> Result<()> {

    let config = match load_config() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("error loading config.toml: {:#}", e);
            return Err(e);
        }
    };

    #[cfg(target_os = "linux")]
    {
        let os = linux_control::LinuxControl::new();
        run_service(os, config).await?;
    }

    #[cfg(all(target_os = "windows", feature = "windows"))]
    {
        let os = windows_control::WindowsControl::new()?;
        run_service(os, config).await?;
    }

    Ok(())
}
