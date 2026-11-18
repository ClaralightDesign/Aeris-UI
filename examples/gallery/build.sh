#!/bin/sh
set -eu
app_root=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
aeris_root=$(CDPATH='' cd -- "$app_root/../.." && pwd)
CANOPUS_ROOT="${CANOPUS_ROOT:-$aeris_root/../Canopus}"
CANOPUS_CLI="${CANOPUS_CLI:-$aeris_root/../.cache/canopus-target/debug/canopus}"
export CANOPUS_ROOT
if [ ! -x "$CANOPUS_CLI" ]; then
  echo "Canopus CLI not found: $CANOPUS_CLI" >&2
  exit 1
fi
version=$(sed -n 's/^version = "\(.*\)"$/\1/p' "$app_root/Canopus.toml")
package="${AERIS_PACKAGE_OUTPUT:-$app_root/dist/aeris_gallery-$version.canopus}"
mkdir -p "$(dirname -- "$package")"
"$CANOPUS_CLI" package --build --manifest "$app_root/Canopus.toml" --output "$package"
"$CANOPUS_CLI" package verify "$package"
