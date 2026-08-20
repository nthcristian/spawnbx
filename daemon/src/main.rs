#[tokio::main]
async fn main() -> anyhow::Result<()> {
    daemon::run_service().await?;
    Ok(())
}
