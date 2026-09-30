#!/bin/sh
printf '%s\0' "$@" >> "$FAKE_ROOT/id.args"
printf '\0' >> "$FAKE_ROOT/id.args"
printf 'id:%s\n' "$1" >> "$FAKE_ROOT/events"
if [ "${FAKE_ID_FAIL-}" = "$1" ]; then
    printf 'fake identity failure\n' >&2
    exit 19
fi
if [ "${FAKE_ID_BAD_UTF8-}" = "$1" ]; then
    printf '\377'
    exit 0
fi
case "$1" in
    -un) printf '%s\n' "${FAKE_USERNAME-test-user}" ;;
    -u) printf '%s\n' "${FAKE_UID-1234}" ;;
    -g) printf '%s\n' "${FAKE_GID-5678}" ;;
    *) exit 98 ;;
esac
