#!/usr/bin/env bash
# Tell repo.rossmel.com that a version was published, so it rebuilds the signed repositories
# with it: push the version to sources/<name> in ross_packages with this repo's deploy key.
#   HUB_DEPLOY_KEY=… notify-hub.sh <name> <version>
set -euo pipefail
name=${1:?name} version=${2:?version}
if [ -z "${HUB_DEPLOY_KEY:-}" ]; then
  echo "::warning::No HUB_DEPLOY_KEY: repo.rossmel.com picks this release up on its daily run"
  exit 0
fi
dir=$(mktemp -d) keyfile=$(mktemp)
trap 'rm -rf "$dir" "$keyfile"' EXIT
printf '%s\n' "$HUB_DEPLOY_KEY" > "$keyfile"
chmod 600 "$keyfile"
export GIT_SSH_COMMAND="ssh -i $keyfile -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new"
git clone -q --depth 1 git@github.com:rossmelabasto/ross_packages.git "$dir"
mkdir -p "$dir/sources"
echo "$version" > "$dir/sources/$name"
git -C "$dir" add sources
git -C "$dir" -c user.name="$name release" -c user.email='41898282+github-actions[bot]@users.noreply.github.com' \
  commit -q --allow-empty -m "$name $version"
git -C "$dir" push -q origin HEAD:main
echo "Told repo.rossmel.com about $name $version"
