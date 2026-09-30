
mkdir -p /etc/profile.d
cat > /etc/profile.d/spawnbx-nix.sh <<'EOF'
case ":$PATH:" in
    *":/nix/var/nix/profiles/default/bin:"*) ;;
    *) export PATH="/nix/var/nix/profiles/default/bin:$PATH" ;;
esac
EOF
chmod 0644 /etc/profile.d/spawnbx-nix.sh
