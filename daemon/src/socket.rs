use std::path::PathBuf;

use tokio::net::UnixListener;

pub async fn create_socket(name: impl AsRef<str>) -> anyhow::Result<(UnixListener, PathBuf)> {
    let name = name.as_ref();

    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")?;
    let path = PathBuf::from(runtime_dir).join(format!("container-ssh/{name}.sock"));

    let _ = tokio::fs::remove_file(&path).await;

    tokio::fs::create_dir_all(
        &path
            .parent()
            .ok_or(anyhow::anyhow!("No parent directory"))?,
    )
    .await?;

    let socket = UnixListener::bind(&path)?;

    Ok((socket, path))
}
