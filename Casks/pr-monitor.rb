cask "pr-monitor" do
  arch arm: "aarch64-apple-darwin", intel: "x86_64-apple-darwin"

  version "0.1.1"
  sha256 arm:   "d6e4facb29c655da94f3de5e12bce80cd1b49f32b04178b3aa72375397a8ee7c",
         intel: "012b533f6e9dde7b222358ce7fb20df8678c6908094e6721f0eef9a5f145fd4d"

  url "https://github.com/guibeira/pr-monitor/releases/download/v#{version}/pr-monitor-#{version}-#{arch}.dmg",
      verified: "github.com/guibeira/pr-monitor/"
  name "PR Monitor"
  desc "Desktop utility for monitoring and updating GitHub pull requests"
  homepage "https://github.com/guibeira/pr-monitor"

  app "Pull request monitor.app"

  zap trash: [
    "~/Library/Application Support/pr-monitor.guibeira.dev",
    "~/Library/Caches/pr-monitor.guibeira.dev",
    "~/Library/Logs/pr-monitor.guibeira.dev",
    "~/Library/Preferences/pr-monitor.guibeira.dev.plist",
    "~/Library/WebKit/pr-monitor.guibeira.dev",
  ]
end
