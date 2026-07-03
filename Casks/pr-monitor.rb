cask "pr-monitor" do
  arch arm: "aarch64-apple-darwin", intel: "x86_64-apple-darwin"

  version "0.1.4"
  sha256 arm:   "0cffaaad3ab4d46821d8a71b7af8e98c64cf2ed9004fabf1420455d4804093fb",
         intel: "358a309bcdd21d279a8be004f43866cfb97145c79bdeec147b303c8de82e652d"

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
