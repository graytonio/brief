# typed: false
# frozen_string_literal: true

class Brief < Formula
  desc "Remote standards manager for Claude Code"
  homepage "https://github.com/graytonio/brief"
  version "0.0.4"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/graytonio/brief/releases/download/v#{version}/brief-macos-aarch64.tar.gz"
      sha256 "56e0c7bfcd6a766652d7f033c383995f7851072425e7abedca8ffcffcf80bc57"
    end
    on_intel do
      url "https://github.com/graytonio/brief/releases/download/v#{version}/brief-macos-x86_64.tar.gz"
      sha256 "3d59852db6d48ad2fc01db589f9fbef1d6b23305fdc8bce8f7f504f54f9669e9"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/graytonio/brief/releases/download/v#{version}/brief-linux-aarch64.tar.gz"
      sha256 "a7da98bf9699bd366f25624aee06881e895a1ff52d274a4742e8a71a7006a4aa"
    end
    on_intel do
      url "https://github.com/graytonio/brief/releases/download/v#{version}/brief-linux-x86_64.tar.gz"
      sha256 "3d1f36fa223dce916d6f36b4dd36f682e36ee09ced9fcf049d51a3612f0a834e"
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
