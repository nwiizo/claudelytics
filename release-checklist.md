# Release Checklist for Homebrew Installation

This checklist ensures that claudelytics is properly set up for Homebrew installation.

## 📋 Pre-Release Checklist

### Code Quality
- [ ] All tests pass (`cargo test`)
- [ ] Code is properly formatted (`cargo fmt --check`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Version bumped in `Cargo.toml`
- [ ] CHANGELOG.md updated

### GitHub Setup
- [ ] Repository is public
- [ ] GitHub Actions workflows are configured
- [ ] Release workflow builds for all platforms:
  - [ ] macOS x86_64 (Intel)
  - [ ] macOS aarch64 (Apple Silicon)
  - [ ] Linux x86_64
  - [ ] Linux aarch64
  - [ ] Windows x86_64

### Documentation
- [ ] README.md includes Homebrew installation instructions
- [ ] INSTALL.md is comprehensive
- [ ] Usage examples are up to date

## 🚀 Release Process

### 1. Create Release

```bash
# 1. Tag the release
git tag v0.1.0
git push origin v0.1.0

# 2. GitHub Actions will automatically:
#    - Build binaries for all platforms
#    - Create GitHub release
#    - Upload release artifacts
```

### 2. Update Homebrew Formula

After the GitHub release is created:

```bash
# 1. Download and get SHA256 checksums for each platform
curl -fsSL https://github.com/nwiizo/claudelytics/releases/download/v0.1.0/claudelytics-x86_64-apple-darwin.tar.gz | shasum -a 256
curl -fsSL https://github.com/nwiizo/claudelytics/releases/download/v0.1.0/claudelytics-aarch64-apple-darwin.tar.gz | shasum -a 256
curl -fsSL https://github.com/nwiizo/claudelytics/releases/download/v0.1.0/claudelytics-x86_64-unknown-linux-gnu.tar.gz | shasum -a 256
curl -fsSL https://github.com/nwiizo/claudelytics/releases/download/v0.1.0/claudelytics-aarch64-unknown-linux-gnu.tar.gz | shasum -a 256

# 2. Update Formula/claudelytics.rb with:
#    - New version number
#    - New SHA256 checksums for each platform
```

### 3. Create Homebrew Tap Repository

```bash
# 1. Create new repository: homebrew-claudelytics
# 2. Copy Formula/claudelytics.rb to the root
# 3. Add README.md for the tap
```

### 4. Test Installation

```bash
# Test the formula locally
brew install --build-from-source ./Formula/claudelytics.rb

# Test that it works
claudelytics --version
claudelytics --help

# Clean up
brew uninstall claudelytics
```

### 5. Publish Tap

```bash
# 1. Push homebrew-claudelytics repository
# 2. Users can then install with:
brew tap nwiizo/claudelytics
brew install claudelytics
```

## ✅ Post-Release Verification

### Installation Methods
- [ ] Homebrew installation works
- [ ] Install script works
- [ ] Manual binary download works
- [ ] Building from source works

### Functionality Testing
- [ ] `claudelytics --version` shows correct version
- [ ] `claudelytics --help` works
- [ ] Basic commands work:
  - [ ] `claudelytics daily`
  - [ ] `claudelytics session`
  - [ ] `claudelytics tui`
  - [ ] `claudelytics config --show`

### Platform Testing
- [ ] macOS Intel (x86_64)
- [ ] macOS Apple Silicon (aarch64)
- [ ] Linux x86_64
- [ ] Linux aarch64
- [ ] Windows x86_64

## 🔧 Formula Template

When updating the Homebrew formula, use this template:

```ruby
class Claudelytics < Formula
  desc "Claude Code usage analytics tool with TUI interface"
  homepage "https://github.com/nwiizo/claudelytics"
  version "X.Y.Z"  # <- Update this
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/nwiizo/claudelytics/releases/download/vX.Y.Z/claudelytics-aarch64-apple-darwin.tar.gz"
      sha256 "SHA256_HERE"  # <- Update this
    else
      url "https://github.com/nwiizo/claudelytics/releases/download/vX.Y.Z/claudelytics-x86_64-apple-darwin.tar.gz"
      sha256 "SHA256_HERE"  # <- Update this
    end
  end

  on_linux do
    if Hardware::CPU.arm? && Hardware::CPU.arch == :arm64
      url "https://github.com/nwiizo/claudelytics/releases/download/vX.Y.Z/claudelytics-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "SHA256_HERE"  # <- Update this
    else
      url "https://github.com/nwiizo/claudelytics/releases/download/vX.Y.Z/claudelytics-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "SHA256_HERE"  # <- Update this
    end
  end

  def install
    bin.install "claudelytics"
  end

  test do
    system "#{bin}/claudelytics", "--version"
    system "#{bin}/claudelytics", "--help"
  end
end
```

## 📞 Support

If users encounter issues:

1. Direct them to [GitHub Issues](https://github.com/nwiizo/claudelytics/issues)
2. Common troubleshooting steps in INSTALL.md
3. Verify their Claude Code setup is correct

## 🔄 Automation Opportunities

Consider automating these steps:

1. **Formula Updates**: Script to update SHA256 checksums automatically
2. **Cross-platform Testing**: CI/CD to test installation on different platforms
3. **Version Synchronization**: Ensure Cargo.toml and Formula versions match
4. **Homebrew Core**: Eventually submit to homebrew-core for wider distribution

## 📚 References

- [Homebrew Formula Cookbook](https://docs.brew.sh/Formula-Cookbook)
- [GitHub Actions for Rust](https://doc.rust-lang.org/cargo/guide/continuous-integration.html)
- [Creating GitHub Releases](https://docs.github.com/en/repositories/releasing-projects-on-github)