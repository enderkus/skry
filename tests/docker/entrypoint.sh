#!/bin/sh
# Test container entrypoint: installs the test public key for the `skry`
# user, generates host keys and runs sshd in the foreground.
set -e
if [ -n "$SKRY_TEST_PUBKEY" ]; then
    mkdir -p /home/skry/.ssh
    printf '%s\n' "$SKRY_TEST_PUBKEY" > /home/skry/.ssh/authorized_keys
    chown -R skry:skry /home/skry/.ssh
    chmod 700 /home/skry/.ssh
    chmod 600 /home/skry/.ssh/authorized_keys
fi
ssh-keygen -A >/dev/null
mkdir -p /run/sshd
exec /usr/sbin/sshd -D -e "$@"
