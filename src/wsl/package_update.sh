set -eu
if command -v apt-get >/dev/null 2>&1; then
    export DEBIAN_FRONTEND=noninteractive
    apt-get update
    apt-get upgrade -y
elif command -v dnf >/dev/null 2>&1; then
    dnf upgrade -y
elif command -v yum >/dev/null 2>&1; then
    yum update -y
elif command -v zypper >/dev/null 2>&1; then
    zypper --non-interactive update
elif command -v pacman >/dev/null 2>&1; then
    pacman -Syu --noconfirm
elif command -v apk >/dev/null 2>&1; then
    apk update
    apk upgrade
else
    printf '%s\n' 'No supported package manager found (apt-get, dnf, yum, zypper, pacman, apk).' >&2
    exit 127
fi
