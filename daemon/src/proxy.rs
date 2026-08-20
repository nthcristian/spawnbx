use std::{collections::HashMap, path::PathBuf};

use tokio::{net::UnixListener, task::JoinHandle};

use crate::{container::get_container_ip, socket};

struct Proxy {
    socket_path: PathBuf,
    task: JoinHandle<()>,
}

impl Drop for Proxy {
    fn drop(&mut self) {
        self.task.abort();
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

pub struct ProxyManager {
    proxies: HashMap<String, Proxy>,
}

impl ProxyManager {
    pub fn new() -> Self {
        Self {
            proxies: HashMap::new(),
        }
    }

    pub async fn create_proxies(
        &mut self,
        names: impl Iterator<Item = impl AsRef<str>>,
    ) -> anyhow::Result<()> {
        for name in names {
            let name = name.as_ref();

            let (socket, socket_path) = socket::create_socket(name).await?;

            let container_name = String::from(name);
            let task = tokio::spawn(async move {
                if let Err(err) = proxy_task(socket, container_name).await {
                    eprintln!("{}", err);
                }
            });

            let proxy = Proxy { task, socket_path };
            self.proxies.insert(name.into(), proxy);
        }

        Ok(())
    }

    pub async fn remove_proxies(
        &mut self,
        names: impl Iterator<Item = impl AsRef<str>>,
    ) -> anyhow::Result<()> {
        for name in names {
            self.proxies.remove(name.as_ref());
        }

        Ok(())
    }
}

async fn proxy_task(socket: UnixListener, container_name: String) -> anyhow::Result<()> {
    loop {
        println!("listening to {}.sock", &container_name);
        let Ok((mut stream, _)) = socket.accept().await else {
            break;
        };

        println!("Connetion on {}.sock, fowarding...", &container_name);

        let container_ip = get_container_ip(&container_name).await?;

        let mut tcp = tokio::net::TcpStream::connect((container_ip.as_str(), 22)).await?;

        tokio::io::copy_bidirectional(&mut stream, &mut tcp).await?;
    }

    Ok(())
}
