FROM --platform=linux/amd64 archlinux@sha256:305558d2bce0b33170f7f7e4ee690633df4b1e0bdfd8194fe45a6545172319ce

LABEL org.opencontainers.image.version="0.1.0" \
      org.opencontainers.image.base.digest="sha256:305558d2bce0b33170f7f7e4ee690633df4b1e0bdfd8194fe45a6545172319ce" \
      org.opencontainers.image.platform="linux/amd64" \
      org.opencontainers.image.nix-version="2.35.2"

RUN pacman -Syu --noconfirm \
        bash \
        coreutils \
        git \
        man-db \
        man-pages \
        nix=2.35.2-2 \
        shadow \
        sudo \
    && pacman -Scc --noconfirm

RUN mkdir -p /etc/nix \
    && printf '%s\n' \
        'experimental-features = nix-command flakes' \
        'sandbox = false' \
        > /etc/nix/nix.conf

# The workload runs as the host UID/GID, so the development image uses a
# writable single-user Nix store instead of requiring a daemon inside PID 1.
RUN chmod -R a+rwX /nix

ENV HOME=/home/spawnbx
WORKDIR /workspace
CMD ["sleep", "infinity"]
