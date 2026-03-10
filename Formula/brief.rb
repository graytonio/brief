# typed: false
# frozen_string_literal: true

class Brief < Formula
  desc "Remote standards manager for Claude Code"
  homepage "https://github.com/graytonio/brief"
  version "0.0.1"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/graytonio/brief/releases/download/v#{version}/brief-macos-aarch64.tar.gz"
      sha256 "42da1c82e7f49f279d19f7cdaca44feda7ab482f427e4c110da2cdae7d780f20"
    end
    on_intel do
      url "https://github.com/graytonio/brief/releases/download/v#{version}/brief-macos-x86_64.tar.gz"
      sha256 "1e56e4bdce8617905f8a724a10a20df8941b58ff2f53fbbdbf773b00d0eaebbf"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/graytonio/brief/releases/download/v#{version}/brief-linux-aarch64.tar.gz"
      sha256 "fc93b597964da3871012cc1537ee2c5395574e8b7bcdcbd3dce81396638eeefd"
    end
    on_intel do
      url "https://github.com/graytonio/brief/releases/download/v#{version}/brief-linux-x86_64.tar.gz"
      sha256 "8ada117d9413c2ffc03d881a3a0cbeb79b7829550570c35f419c3e0e23190feb"
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
