cask "pr-monitor" do
  arch arm: "aarch64-apple-darwin", intel: "x86_64-apple-darwin"

  version "0.1.3"
  sha256 arm:   "bac7db0809504633fd3c466c6b9014e069525c86450761e35f52f588013d2944",
         intel: "e173b0ff0352acf06dad4b61fd62b857edb399cbf7f30010053d281939650b9b"

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
