# Installation Guide

This guide covers multiple ways to install claudelytics on your system.

## 🍺 Homebrew (Recommended for macOS/Linux)

The easiest way to install claudelytics on macOS and Linux is using Homebrew.

### Prerequisites

- Homebrew must be installed on your system
- For installation instructions, visit: https://brew.sh/

### Installation

```bash
# Add the claudelytics tap
brew tap nwiizo/claudelytics

# Install claudelytics
brew install claudelytics

# Verify installation
claudelytics --version
```

### Updating

```bash
brew update
brew upgrade claudelytics
```

### Uninstalling

```bash
brew uninstall claudelytics
brew untap nwiizo/claudelytics
```

## ⚡ Quick Install Script

For a quick installation without Homebrew:

```bash
# Install latest version
curl -fsSL https://raw.githubusercontent.com/nwiizo/claudelytics/main/install.sh | bash

# Or with custom install directory
CLAUDELYTICS_INSTALL_DIR=/usr/local/bin curl -fsSL https://raw.githubusercontent.com/nwiizo/claudelytics/main/install.sh | bash
```

### What the script does:

1. Detects your operating system and architecture
2. Downloads the appropriate binary for your platform
3. Installs it to `~/.local/bin` (or `/usr/local/bin` if run as root)
4. Makes it executable
5. Provides instructions for adding to PATH if needed

## 📦 Manual Installation from GitHub Releases

### Step 1: Download

Visit the [Releases page](https://github.com/nwiizo/claudelytics/releases) and download the appropriate binary for your platform:

- **macOS (Intel)**: `claudelytics-x86_64-apple-darwin.tar.gz`
- **macOS (Apple Silicon)**: `claudelytics-aarch64-apple-darwin.tar.gz`
- **Linux (x86_64)**: `claudelytics-x86_64-unknown-linux-gnu.tar.gz`
- **Linux (ARM64)**: `claudelytics-aarch64-unknown-linux-gnu.tar.gz`
- **Windows**: `claudelytics-x86_64-pc-windows-msvc.zip`

### Step 2: Extract

```bash
# For tar.gz files
tar -xzf claudelytics-*.tar.gz

# For zip files (Windows)
unzip claudelytics-*.zip
```

### Step 3: Install

```bash
# Move to a directory in your PATH
sudo mv claudelytics /usr/local/bin/

# Or to user directory
mkdir -p ~/.local/bin
mv claudelytics ~/.local/bin/

# Make executable (Unix systems)
chmod +x /usr/local/bin/claudelytics
```

### Step 4: Add to PATH (if using ~/.local/bin)

Add to your shell profile (`~/.bashrc`, `~/.zshrc`, etc.):

```bash
export PATH="$HOME/.local/bin:$PATH"
```

Then reload your shell:

```bash
source ~/.bashrc  # or ~/.zshrc
```

## 🔧 Build from Source

### Prerequisites

- Rust toolchain (1.70.0 or later)
- Git

### Installation

```bash
# Install Rust if you haven't already
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Clone the repository
git clone https://github.com/nwiizo/claudelytics.git
cd claudelytics

# Build in release mode
cargo build --release

# Install to cargo's bin directory
cargo install --path .
```

The binary will be installed to `~/.cargo/bin/claudelytics` (make sure `~/.cargo/bin` is in your PATH).

### Development Build

For development purposes:

```bash
# Build in debug mode (faster compilation)
cargo build

# Run directly
cargo run -- --help

# Run tests
cargo test
```

## 🐳 Docker

You can also run claudelytics in a Docker container:

```bash
# Build the image
docker build -t claudelytics .

# Run with your Claude data mounted
docker run --rm -v ~/.claude:/root/.claude claudelytics daily
```

## ✅ Verify Installation

After installation, verify that claudelytics is working:

```bash
# Check version
claudelytics --version

# Show help
claudelytics --help

# Run a basic command
claudelytics config --show
```

## 🔧 Configuration

After installation, you may want to configure claudelytics:

```bash
# Show current configuration
claudelytics config --show

# Set custom Claude directory path
claudelytics config --set-path /path/to/claude

# Reset configuration to defaults
claudelytics config --reset
```

## 🚀 Getting Started

Once installed, try these commands:

```bash
# View daily usage
claudelytics daily

# Launch interactive TUI
claudelytics tui

# View session analytics
claudelytics session

# Show today's cost
claudelytics cost --today
```

## 🆘 Troubleshooting

### Command not found

If you get "command not found" after installation:

1. Check if the binary is in your PATH:
   ```bash
   echo $PATH
   which claudelytics
   ```

2. Add the installation directory to your PATH:
   ```bash
   export PATH="/path/to/claudelytics:$PATH"
   ```

3. Make sure the binary is executable:
   ```bash
   chmod +x /path/to/claudelytics
   ```

### Permission denied

If you get permission errors:

1. Make sure the binary is executable:
   ```bash
   chmod +x /path/to/claudelytics
   ```

2. Check directory permissions:
   ```bash
   ls -la /path/to/claudelytics
   ```

### No data found

If claudelytics reports no data:

1. Verify Claude Code has been used and data exists:
   ```bash
   ls ~/.claude/projects/
   ```

2. Check the Claude directory path:
   ```bash
   claudelytics config --show
   ```

3. Set the correct path if needed:
   ```bash
   claudelytics config --set-path /correct/path/to/claude
   ```

## 📚 Next Steps

- Read the [User Guide](./USAGE.md) for detailed usage instructions
- Check out [Examples](./examples/) for common use cases
- Visit the [GitHub repository](https://github.com/nwiizo/claudelytics) for updates and issues

## 🤝 Contributing

If you'd like to contribute to claudelytics, see [CONTRIBUTING.md](./CONTRIBUTING.md) for development setup and guidelines.