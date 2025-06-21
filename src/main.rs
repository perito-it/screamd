mod os_control;
mod service_core;

#[cfg(target_os = "linux")]
mod linux_control;
#[cfg(target_os = "windows")]
mod windows_control;

use anyhow::Result;
use service_core::run_service;

fn load_config() -> anyhow::Result<service_core::Config> {
    let config_str = std::fs::read_to_string("config/config.toml")?;
    let config: service_core::Config = toml::from_str(&config_str)?;
    Ok(config)
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = load_config()?;

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
