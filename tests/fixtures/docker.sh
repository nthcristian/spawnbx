#!/bin/sh
# Record arguments as data only. Never execute the command supplied to docker exec.
printf '%s\0' "$@" >> "$FAKE_ROOT/docker.args"
printf '\0' >> "$FAKE_ROOT/docker.args"

case "$1:$2" in
    container:inspect) phase=inspect ;;
    container:start) phase=start ;;
    container:stop) phase=stop ;;
    run:*) phase=create ;;
    rm:*) phase=remove ;;
    exec:--interactive) phase=attach ;;
    exec:*)
        case "$3:$4:$5" in
            sh:-eu:-c)
                case "$6" in
                    *'username="$1"'*) phase=user ;;
                    *'/etc/profile.d/spawnbx-nix.sh'*) phase=path ;;
                    *) printf 'unexpected script' >&2; exit 98 ;;
                esac ;;
            nix:profile:list) phase=profile ;;
            nix:profile:upgrade) phase=upgrade ;;
            nix:profile:install) phase=install ;;
            *) printf 'unexpected exec' >&2; exit 98 ;;
        esac ;;
    *) printf 'unexpected docker invocation' >&2; exit 98 ;;
esac
home=absent
[ ! -d .spawnbx/home ] || home=present
printf '%s:%s\n' "$phase" "$home" >> "$FAKE_ROOT/events"
if [ ! -f "$FAKE_ROOT/first-docker-settings" ]; then
    while IFS= read -r line; do
        printf '%s\n' "$line"
    done < .spawnbx.yml > "$FAKE_ROOT/first-docker-settings"
fi

if [ "${FAKE_SIGNAL_PHASE-}" = "$phase" ]; then
    kill -TERM "$$"
fi
if [ "${FAKE_BAD_STDOUT_PHASE-}" = "$phase" ]; then
    printf '\377'
    exit 0
fi
if [ "${FAKE_BAD_STDERR_PHASE-}" = "$phase" ]; then
    printf '\377' >&2
    exit 0
fi
if [ "${FAKE_FAIL_PHASE-}" = "$phase" ]; then
    printf 'fake %s failure\n' "$phase" >&2
    exit "${FAKE_FAIL_STATUS-17}"
fi
case "$phase" in
    inspect)
        case "${FAKE_INSPECT-true}" in
            missing) printf 'Error: No such container: fake\n' >&2; exit 1 ;;
            wrong-case) printf 'no such container: fake\n' >&2; exit 1 ;;
            *) printf '%s' "${FAKE_INSPECT-true}" ;;
        esac ;;
    profile) printf '%s' "${FAKE_PROFILE-\{"elements":\{\}\}}" ;;
    attach)
        printf '%s' "${FAKE_ATTACH_STDOUT-}"
        printf '%s' "${FAKE_ATTACH_STDERR-}" >&2 ;;
esac
