use async_trait::async_trait;
use tokio::time::Duration;

#[async_trait]
pub trait OsControl: Send + Sync + 'static {
    async fn show_warning(&self, message: &str);
    fn set_login_banner(&self, message: Option<&str>) -> anyhow::Result<()>;
    async fn reboot(&self) -> anyhow::Result<()>;
    async fn shutdown(&self) -> anyhow::Result<()>;
    fn warn_interval(&self) -> Duration;
}
