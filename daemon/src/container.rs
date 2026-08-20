use tokio::process::Command;

pub async fn list_running_containers(label: impl AsRef<str>) -> anyhow::Result<Vec<String>> {
    let label = label.as_ref();

    let output = Command::new("podman")
        .args([
            "ps",
            "--format",
            "{{.Names}}",
            "--filter",
            &format!("label={label}"),
        ])
        .output()
        .await?;

    anyhow::ensure!(output.status.success(), String::from_utf8(output.stderr)?);

    let stdout = String::from_utf8(output.stdout)?;
    let stdout = stdout.trim();

    if stdout.is_empty() {
        return Ok(Vec::with_capacity(0));
    }

    let result = stdout.split("\n").map(String::from).collect();

    Ok(result)
}

pub async fn get_container_ip(name: impl AsRef<str>) -> anyhow::Result<String> {
    let output = tokio::process::Command::new("podman")
        .args([
            "inspect",
            "--format",
            "{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}",
            name.as_ref(),
        ])
        .output()
        .await?;

    anyhow::ensure!(output.status.success(), String::from_utf8(output.stderr)?);

    let stdout = String::from_utf8(output.stdout)?;
    anyhow::ensure!(!stdout.is_empty(), "ip retreive failed");

    Ok(stdout.trim().to_owned())
}
