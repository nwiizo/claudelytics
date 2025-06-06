class Claudelytics < Formula
  desc "Claude Code usage analytics tool with TUI interface"
  homepage "https://github.com/nwiizo/claudelytics"
  version "0.3.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/nwiizo/claudelytics/releases/download/v#{version}/claudelytics-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_ARM64_SHA256"
    else
      url "https://github.com/nwiizo/claudelytics/releases/download/v#{version}/claudelytics-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_X86_64_SHA256"
    end
  end

  on_linux do
    if Hardware::CPU.arm? && Hardware::CPU.arch == :arm64
      url "https://github.com/nwiizo/claudelytics/releases/download/v#{version}/claudelytics-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_LINUX_ARM64_SHA256"
    else
      url "https://github.com/nwiizo/claudelytics/releases/download/v#{version}/claudelytics-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_LINUX_X86_64_SHA256"
    end
  end

  def install
    bin.install "claudelytics"
    
    # Install shell completions
    generate_completions_from_executable(bin/"claudelytics", "completion")
    
    # Install man page if it exists
    if (buildpath/"claudelytics.1").exist?
      man1.install "claudelytics.1"
    end
  end

  test do
    # Test basic functionality
    system "#{bin}/claudelytics", "--version"
    
    # Test help command
    system "#{bin}/claudelytics", "--help"
    
    # Test config command
    system "#{bin}/claudelytics", "config", "--show"
  end

  def caveats
    <<~EOS
      claudelytics analyzes Claude Code usage from ~/.claude/projects/

      To get started:
        1. Run 'claudelytics daily' to see daily usage report
        2. Run 'claudelytics session' to see session-based report
        3. Run 'claudelytics tui' for interactive terminal interface
        4. Run 'claudelytics --help' for all available commands

      Configuration file is stored at: ~/.config/claudelytics/config.yaml

      For more information, visit: https://github.com/nwiizo/claudelytics
    EOS
  end
end