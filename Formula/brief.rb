# typed: false
# frozen_string_literal: true

class Brief < Formula
  desc "Remote standards manager for Claude Code"
  homepage "https://github.com/graytonio/brief"
  version "0.0.3"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/graytonio/brief/releases/download/v#{version}/brief-macos-aarch64.tar.gz"
      sha256 "3d37fc1bb74cc5b9644ac898cb99db17a881e5390bbcfd4dfd1b36d511958fb9"
    end
    on_intel do
      url "https://github.com/graytonio/brief/releases/download/v#{version}/brief-macos-x86_64.tar.gz"
      sha256 "921b543b7935d6ecbff20dcaaafc5f79a72d69548ae46c3a80328f15f2bb1e49"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/graytonio/brief/releases/download/v#{version}/brief-linux-aarch64.tar.gz"
      sha256 "d4c41124937fe33d65665e3ff3baee16bfa0f484c3704099b50e86a5b43086bd"
    end
    on_intel do
      url "https://github.com/graytonio/brief/releases/download/v#{version}/brief-linux-x86_64.tar.gz"
      sha256 "ef79e59e06fa452d411bcfd7787bc85915b8f1a1ae98e71e0217e794aae1e915"
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
