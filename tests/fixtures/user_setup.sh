
username="$1"
uid="$2"
gid="$3"

if id -- "$username" >/dev/null 2>&1; then
    [ "$(id -u -- "$username")" = "$uid" ]
    [ "$(id -g -- "$username")" = "$gid" ]
else
    if ! getent group "$gid" >/dev/null; then
        groupadd --gid "$gid" -- "$username"
    fi

    useradd --uid "$uid" --gid "$gid" --home-dir /home/spawnbx \
        --create-home --shell /bin/bash -- "$username"
fi

usermod --append --groups wheel -- "$username"
printf '%s ALL=(ALL) NOPASSWD: ALL\n' "$username" > /etc/sudoers.d/spawnbx
chmod 0440 /etc/sudoers.d/spawnbx

chown -R $username:$username $HOME
