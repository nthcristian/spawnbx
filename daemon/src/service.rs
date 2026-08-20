use std::{collections::HashSet, time::Duration};

use crate::{container::list_running_containers, proxy::ProxyManager};
use tokio::signal::unix::{SignalKind, signal};

const LABEL: &'static str = "DEV_CONTAINER";

pub async fn run_service() -> anyhow::Result<()> {
    let mut proxies = ProxyManager::new();
    let mut watch_list = HashSet::<String>::new();

    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;

    let shutdown = async {
        tokio::select! {
            _ = sigterm.recv() => {},
            _ = sigint.recv() => {},
        }
    };
    tokio::pin!(shutdown);

    loop {
        let poll = async {
            let running_containers: HashSet<String> = list_running_containers(LABEL)
                .await?
                .iter()
                .map(String::from)
                .collect();

            let deleted_containers = watch_list.difference(&running_containers);
            proxies.remove_proxies(deleted_containers).await?;

            let new_containers: Vec<String> = running_containers
                .difference(&watch_list)
                .cloned()
                .collect();

            proxies.create_proxies(new_containers.iter()).await?;

            watch_list = running_containers;

            tokio::time::sleep(Duration::from_secs(5)).await;
            Ok::<(), anyhow::Error>(())
        };

        tokio::select! {
            result = poll => result?,
            _ = &mut shutdown => {
                println!("shutdown signal received");
                break;
            },
        }
    }

    Ok(())
}
