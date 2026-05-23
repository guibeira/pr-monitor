cask "pr-monitor" do
  arch arm: "aarch64-apple-darwin", intel: "x86_64-apple-darwin"

  version "0.1.2"
  sha256 arm:   "2426468b65f1270b09f28cf303ab7d733606154312ca102412d6afa5655c145b",
         intel: "8c32bd1c21e5bb62ba532b08efddf604fe58060cbd0c028cdded146c027010bd"

  url "https://github.com/guibeira/pr-monitor/releases/download/v#{version}/pr-monitor-#{version}-#{arch}.dmg",
      verified: "github.com/guibeira/pr-monitor/"
  name "PR Monitor"
  desc "Desktop utility for monitoring and updating GitHub pull requests"
  homepage "https://github.com/guibeira/pr-monitor"

  app "Pull request monitor.app"

  postflight do
    system_command "/usr/bin/xattr",
                   args: ["-d", "com.apple.quarantine", "#{appdir}/Pull request monitor.app"],
                   sudo: false
  end

  zap trash: [
    "~/Library/Application Support/pr-monitor.guibeira.dev",
    "~/Library/Caches/pr-monitor.guibeira.dev",
    "~/Library/Logs/pr-monitor.guibeira.dev",
    "~/Library/Preferences/pr-monitor.guibeira.dev.plist",
    "~/Library/WebKit/pr-monitor.guibeira.dev",
  ]
end
