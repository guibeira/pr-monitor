#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 3 ]; then
  echo "usage: $0 <version> <checksums_file> <output_file> [owner] [repo]" >&2
  exit 1
fi

version="$1"
checksums_file="$2"
output_file="$3"
owner="${4:-guibeira}"
repo="${5:-pr-monitor}"

if [ ! -f "${checksums_file}" ]; then
  echo "checksums file not found: ${checksums_file}" >&2
  exit 1
fi

darwin_x64_dmg="pr-monitor-${version}-x86_64-apple-darwin.dmg"
darwin_arm64_dmg="pr-monitor-${version}-aarch64-apple-darwin.dmg"

lookup_sha() {
  local asset_name="$1"
  awk -v target="${asset_name}" '$2 == target { print $1 }' "${checksums_file}"
}

darwin_x64_sha="$(lookup_sha "${darwin_x64_dmg}")"
darwin_arm64_sha="$(lookup_sha "${darwin_arm64_dmg}")"

test -n "${darwin_x64_sha}"
test -n "${darwin_arm64_sha}"

mkdir -p "$(dirname "${output_file}")"

cat > "${output_file}" <<EOF
cask "pr-monitor" do
  arch arm: "aarch64-apple-darwin", intel: "x86_64-apple-darwin"

  version "${version}"
  sha256 arm:   "${darwin_arm64_sha}",
         intel: "${darwin_x64_sha}"

  url "https://github.com/${owner}/${repo}/releases/download/v#{version}/pr-monitor-#{version}-#{arch}.dmg",
      verified: "github.com/${owner}/${repo}/"
  name "PR Monitor"
  desc "Desktop utility for monitoring and updating GitHub pull requests"
  homepage "https://github.com/${owner}/${repo}"

  app "Pull request monitor.app"

  zap trash: [
    "~/Library/Application Support/pr-monitor.guibeira.dev",
    "~/Library/Caches/pr-monitor.guibeira.dev",
    "~/Library/Logs/pr-monitor.guibeira.dev",
    "~/Library/Preferences/pr-monitor.guibeira.dev.plist",
    "~/Library/WebKit/pr-monitor.guibeira.dev",
  ]
end
EOF
