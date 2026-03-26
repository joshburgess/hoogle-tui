# Homebrew formula for hoogle-tui
# To use: create a tap repo (homebrew-hoogle-tui) and place this as Formula/hoogle-tui.rb
#
# Users install with:
#   brew tap joshburgess/hoogle-tui
#   brew install hoogle-tui

class HoogleTui < Formula
  desc "Terminal UI for Haskell's Hoogle search engine"
  homepage "https://github.com/joshburgess/hoogle-tui"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/joshburgess/hoogle-tui/releases/download/v#{version}/hoogle-tui-aarch64-apple-darwin.tar.gz"
      # sha256 "PLACEHOLDER" # Update after release
    end
    on_intel do
      url "https://github.com/joshburgess/hoogle-tui/releases/download/v#{version}/hoogle-tui-x86_64-apple-darwin.tar.gz"
      # sha256 "PLACEHOLDER" # Update after release
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/joshburgess/hoogle-tui/releases/download/v#{version}/hoogle-tui-aarch64-unknown-linux-gnu.tar.gz"
      # sha256 "PLACEHOLDER" # Update after release
    end
    on_intel do
      url "https://github.com/joshburgess/hoogle-tui/releases/download/v#{version}/hoogle-tui-x86_64-unknown-linux-musl.tar.gz"
      # sha256 "PLACEHOLDER" # Update after release
    end
  end

  def install
    bin.install "hoogle-tui"

    # Install shell completions if present
    if File.exist?("completions/hoogle-tui.bash")
      bash_completion.install "completions/hoogle-tui.bash" => "hoogle-tui"
    end
    if File.exist?("completions/hoogle-tui.zsh")
      zsh_completion.install "completions/hoogle-tui.zsh" => "_hoogle-tui"
    end
    if File.exist?("completions/hoogle-tui.fish")
      fish_completion.install "completions/hoogle-tui.fish"
    end
  end

  def caveats
    <<~EOS
      hoogle-tui works best with a local Hoogle database:
        cabal install hoogle
        hoogle generate

      Or generate via hoogle-tui itself:
        hoogle-tui --generate

      Without local Hoogle, the web API at hoogle.haskell.org is used as fallback.
    EOS
  end

  test do
    assert_match "hoogle-tui", shell_output("#{bin}/hoogle-tui --version")
  end
end
