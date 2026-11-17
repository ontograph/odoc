#!/bin/sh
# Install the Linux x86_64 GNU release from https://github.com/ontograph/odoc.
set -eu

release=latest
bin_dir=${HOME:-}/.local/bin
while [ "$#" -gt 0 ]; do
    case "$1" in
        --version|--bin-dir)
            [ "$#" -ge 2 ] || { printf 'Missing value for %s\n' "$1" >&2; exit 1; }
            case "$1" in
                --version) release=$2 ;;
                --bin-dir) bin_dir=$2 ;;
            esac
            shift 2 ;;
        -h|--help)
            printf '%s\n' 'Usage: sh install.sh [--version 0.1.2] [--bin-dir /path/to/bin]' \
                'Defaults: latest release; $HOME/.local/bin. Existing binaries are backed up.'
            exit 0 ;;
        *) printf 'Unknown argument: %s\n' "$1" >&2; exit 1 ;;
    esac
done

case "$bin_dir" in
    /*) ;;
    *) printf '%s\n' '--bin-dir must be an absolute path.' >&2; exit 1 ;;
esac
[ -n "${HOME:-}" ] || [ "$bin_dir" != '/.local/bin' ] || {
    printf '%s\n' 'Set HOME or pass --bin-dir.' >&2; exit 1;
}
if [ "$(uname -s)" != Linux ] || [ "$(uname -m)" != x86_64 ]; then
    printf '%s\n' 'Supported platform: Linux x86_64 with glibc 2.39 or newer.' >&2
    exit 1
fi
for command in curl sha256sum tar mktemp install getconf awk cmp; do
    command -v "$command" >/dev/null 2>&1 || {
        printf 'Required command is missing: %s\n' "$command" >&2; exit 1;
    }
done
glibc=$(getconf GNU_LIBC_VERSION 2>/dev/null) || {
    printf '%s\n' 'The release requires glibc 2.39 or newer.' >&2; exit 1;
}
printf '%s\n' "$glibc" | awk '
    $1 == "glibc" { split($2, v, "."); if (v[1] > 2 || (v[1] == 2 && v[2] >= 39)) ok=1 }
    END { exit !ok }
' || { printf '%s\n' 'The release requires glibc 2.39 or newer.' >&2; exit 1; }

repo=https://github.com/ontograph/odoc
if [ "$release" = latest ]; then
    latest_url=$(curl --fail --silent --show-error --location --proto '=https' \
        --tlsv1.2 --connect-timeout 15 --max-time 180 --retry 2 \
        --output /dev/null --write-out '%{url_effective}' "$repo/releases/latest")
    release=${latest_url##*/}
fi
release=${release#v}
printf '%s\n' "$release" | awk '
    /^[0-9]+\.[0-9]+\.[0-9]+$/ { ok=1 }
    END { exit !(ok && NR == 1) }
' || { printf '%s\n' 'Expected a release version such as 0.1.2.' >&2; exit 1; }

work=$(mktemp -d)
next=
trap 'rm -rf -- "$work"; if [ -n "$next" ]; then rm -f -- "$next"; fi' EXIT
trap 'exit 1' HUP INT TERM
asset=ontodoc-$release-linux-x86_64-gnu.tar.gz
base=$repo/releases/download/v$release
printf 'Downloading OntoDoc %s...\n' "$release"
curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 \
    --connect-timeout 15 --max-time 180 --retry 2 --max-filesize 33554432 \
    --output "$work/$asset" "$base/$asset"
curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 \
    --connect-timeout 15 --max-time 180 --retry 2 --max-filesize 65536 \
    --output "$work/SHA256SUMS" "$base/SHA256SUMS"
expected=$(awk -v name="$asset" '
    $2 == name { count++; digest=$1 }
    END {
        if (count != 1 || length(digest) != 64 || digest !~ /^[[:xdigit:]]+$/) exit 1
        print digest
    }
' "$work/SHA256SUMS") || { printf '%s\n' 'Missing or invalid archive checksum.' >&2; exit 1; }
printf '%s  %s\n' "$expected" "$work/$asset" | sha256sum --check --status - || {
    printf '%s\n' 'Archive checksum verification failed.' >&2; exit 1;
}
# Extract only the binary to stdout, so archive paths cannot write outside work.
tar -xzOf "$work/$asset" "ontodoc-$release/bin/ontodoc" > "$work/ontodoc"
chmod 755 "$work/ontodoc"
[ "$("$work/ontodoc" --version)" = "ontodoc $release" ] || {
    printf '%s\n' 'Downloaded binary version does not match the release.' >&2; exit 1;
}
mkdir -p -- "$bin_dir"
destination=$bin_dir/ontodoc
[ ! -d "$destination" ] || { printf 'Refusing directory: %s\n' "$destination" >&2; exit 1; }
if [ -e "$destination" ] && cmp -s "$work/ontodoc" "$destination"; then
    printf 'OntoDoc %s is already installed at %s\n' "$release" "$destination"
    exit 0
fi
next=$(mktemp "$bin_dir/.ontodoc.XXXXXX")
install -m 755 "$work/ontodoc" "$next"
if [ -e "$destination" ] || [ -L "$destination" ]; then
    backup=$destination.bak-$(date +%Y%m%d%H%M%S)-$$
    cp -P -- "$destination" "$backup"
    printf 'Previous installation saved at %s\n' "$backup"
fi
mv -f -- "$next" "$destination"
next=
printf 'Installed OntoDoc %s at %s\n' "$release" "$destination"
case ":${PATH:-}:" in
    *:"$bin_dir":*) ;;
    *) printf 'Add %s to PATH to run ontodoc.\n' "$bin_dir" ;;
esac
