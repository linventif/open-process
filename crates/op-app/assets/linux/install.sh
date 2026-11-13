#!/bin/sh
# Install this extracted package for the current user, without root access.
set -eu
package_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
prefix="${OP_INSTALL_PREFIX:-$HOME/.local}"
bin_dir="$prefix/bin"
data_dir="${XDG_DATA_HOME:-$prefix/share}"
install -Dm755 "$package_dir/open-process" "$bin_dir/open-process"
install -Dm644 "$package_dir/open-process.png" "$data_dir/icons/hicolor/512x512/apps/open-process.png"
install -Dm644 "$package_dir/open-process.svg" "$data_dir/icons/hicolor/scalable/apps/open-process.svg"
mkdir -p "$data_dir/applications"
# Quote the absolute executable path according to the Desktop Entry specification.
escaped_exec=$(printf '%s' "$bin_dir/open-process" | sed 's/\\/\\\\\\\\/g; s/[$`"]/\\\\&/g; s/%/%%/g')
while IFS= read -r line; do
    case "$line" in
        Exec=*) printf 'Exec="%s" %%F\n' "$escaped_exec" ;;
        *) printf '%s\n' "$line" ;;
    esac
done < "$package_dir/open-process.desktop" > "$data_dir/applications/open-process.desktop"
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "$data_dir/icons/hicolor" >/dev/null 2>&1 || true
fi
printf 'Open Process installed in %s\n' "$bin_dir"
