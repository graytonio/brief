# typed: false
# frozen_string_literal: true

class Brief < Formula
  desc "Remote standards manager for Claude Code"
  homepage "https://github.com/graytonw/brief"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/graytonw/brief/releases/download/v#{version}/brief-macos-aarch64.tar.gz"
      sha256 "PLACEHOLDER_MACOS_AARCH64_SHA256"
    end
    on_intel do
      url "https://github.com/graytonw/brief/releases/download/v#{version}/brief-macos-x86_64.tar.gz"
      sha256 "PLACEHOLDER_MACOS_X86_64_SHA256"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/graytonw/brief/releases/download/v#{version}/brief-linux-aarch64.tar.gz"
      sha256 "PLACEHOLDER_LINUX_AARCH64_SHA256"
    end
    on_intel do
      url "https://github.com/graytonw/brief/releases/download/v#{version}/brief-linux-x86_64.tar.gz"
      sha256 "PLACEHOLDER_LINUX_X86_64_SHA256"
    end
  end

  def install
    bin.install "brief"
  end

  def caveats
    <<~EOS
      Run the following to finish setup:

        brief init

      This creates ~/.brief/config.toml and installs the Claude Code SessionStart hook.

      To onboard from a team config URL:

        brief init --team-config <url>
    EOS
  end

  test do
    assert_match "brief", shell_output("#{bin}/brief --version")
  end
end
