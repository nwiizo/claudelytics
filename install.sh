#!/usr/bin/env bash

# Claudelytics installer script
# Usage: curl -fsSL https://raw.githubusercontent.com/nwiizo/claudelytics/main/install.sh | bash

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Print colored output
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Detect OS and architecture
detect_platform() {
    local os=""
    local arch=""
    
    case "$OSTYPE" in
        linux-gnu*)
            os="linux"
            ;;
        darwin*)
            os="darwin"
            ;;
        msys*)
            os="windows"
            ;;
        *)
            print_error "Unsupported operating system: $OSTYPE"
            exit 1
            ;;
    esac
    
    case "$(uname -m)" in
        x86_64|amd64)
            arch="x86_64"
            ;;
        aarch64|arm64)
            arch="aarch64"
            ;;
        *)
            print_error "Unsupported architecture: $(uname -m)"
            exit 1
            ;;
    esac
    
    echo "${arch}-${os}"
}

# Get latest release version from GitHub
get_latest_version() {
    local latest_version
    latest_version=$(curl -fsSL "https://api.github.com/repos/nwiizo/claudelytics/releases/latest" | grep -o '"tag_name": "[^"]*' | cut -d'"' -f4)
    
    if [[ -z "$latest_version" ]]; then
        print_error "Failed to get latest version"
        exit 1
    fi
    
    echo "$latest_version"
}

# Download and install claudelytics
install_claudelytics() {
    local platform="$1"
    local version="$2"
    local install_dir="${3:-$HOME/.local/bin}"
    
    # Create install directory if it doesn't exist
    mkdir -p "$install_dir"
    
    # Determine target name based on platform
    local target=""
    case "$platform" in
        x86_64-linux)
            target="x86_64-unknown-linux-gnu"
            ;;
        aarch64-linux)
            target="aarch64-unknown-linux-gnu"
            ;;
        x86_64-darwin)
            target="x86_64-apple-darwin"
            ;;
        aarch64-darwin)
            target="aarch64-apple-darwin"
            ;;
        *)
            print_error "Unsupported platform: $platform"
            exit 1
            ;;
    esac
    
    local download_url="https://github.com/nwiizo/claudelytics/releases/download/${version}/claudelytics-${target}.tar.gz"
    local temp_dir=$(mktemp -d)
    local archive_file="${temp_dir}/claudelytics.tar.gz"
    
    print_info "Downloading claudelytics ${version} for ${platform}..."
    
    if ! curl -fsSL "$download_url" -o "$archive_file"; then
        print_error "Failed to download claudelytics from $download_url"
        exit 1
    fi
    
    print_info "Extracting archive..."
    tar -xzf "$archive_file" -C "$temp_dir"
    
    print_info "Installing to $install_dir..."
    cp "${temp_dir}/claudelytics" "$install_dir/"
    chmod +x "${install_dir}/claudelytics"
    
    # Cleanup
    rm -rf "$temp_dir"
    
    print_success "claudelytics installed successfully to ${install_dir}/claudelytics"
}

# Check if directory is in PATH
check_path() {
    local dir="$1"
    if [[ ":$PATH:" == *":$dir:"* ]]; then
        return 0
    else
        return 1
    fi
}

# Main installation function
main() {
    print_info "Installing claudelytics..."
    
    # Check dependencies
    for cmd in curl tar; do
        if ! command -v "$cmd" &> /dev/null; then
            print_error "$cmd is required but not installed"
            exit 1
        fi
    done
    
    # Detect platform
    local platform
    platform=$(detect_platform)
    print_info "Detected platform: $platform"
    
    # Get latest version
    local version
    version=$(get_latest_version)
    print_info "Latest version: $version"
    
    # Determine install directory
    local install_dir
    if [[ "$EUID" -eq 0 ]]; then
        install_dir="/usr/local/bin"
    else
        install_dir="$HOME/.local/bin"
    fi
    
    # Allow user to override install directory
    if [[ -n "${CLAUDELYTICS_INSTALL_DIR:-}" ]]; then
        install_dir="$CLAUDELYTICS_INSTALL_DIR"
    fi
    
    print_info "Install directory: $install_dir"
    
    # Install claudelytics
    install_claudelytics "$platform" "$version" "$install_dir"
    
    # Check if install directory is in PATH
    if ! check_path "$install_dir"; then
        print_warning "⚠️  $install_dir is not in your PATH"
        print_info "Add the following line to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
        echo "    export PATH=\"$install_dir:\$PATH\""
        print_info "Then reload your shell or run: source ~/.bashrc (or ~/.zshrc)"
    fi
    
    # Verify installation
    if command -v claudelytics &> /dev/null; then
        print_success "✅ Installation completed successfully!"
        print_info "Run 'claudelytics --help' to get started"
    else
        print_warning "⚠️  Installation completed, but 'claudelytics' command not found in PATH"
        print_info "You may need to add $install_dir to your PATH or run:"
        echo "    $install_dir/claudelytics --help"
    fi
    
    # Print quick start guide
    echo ""
    print_info "Quick start:"
    echo "  claudelytics daily     # Show daily usage report"
    echo "  claudelytics session   # Show session-based report"
    echo "  claudelytics tui       # Launch interactive TUI"
    echo "  claudelytics --help    # Show all available commands"
    echo ""
    print_info "For more information, visit: https://github.com/nwiizo/claudelytics"
}

# Run the main function
main "$@"