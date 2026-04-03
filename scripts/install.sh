#!/usr/bin/env bash
set -euo pipefail

# Claudelytics install script
# https://github.com/nwiizo/claudelytics

# Default values
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
GITHUB_REPO="nwiizo/claudelytics"
PROGRAM_NAME="claudelytics"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
print_error() {
    echo -e "${RED}Error: $1${NC}" >&2
}

print_success() {
    echo -e "${GREEN}$1${NC}"
}

print_info() {
    echo -e "${BLUE}$1${NC}"
}

print_warning() {
    echo -e "${YELLOW}$1${NC}"
}

# Check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Detect OS and architecture
detect_platform() {
    local os arch

    # Detect OS
    case "$(uname -s)" in
        Linux*)
            os="linux"
            ;;
        Darwin*)
            os="darwin"
            ;;
        MINGW*|MSYS*|CYGWIN*)
            os="windows"
            ;;
        *)
            print_error "Unsupported operating system: $(uname -s)"
            exit 1
            ;;
    esac

    # Detect architecture
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

    # Special handling for macOS
    if [[ "$os" == "darwin" ]]; then
        os="apple-darwin"
    elif [[ "$os" == "linux" ]]; then
        # Check if musl or gnu
        if ldd --version 2>&1 | grep -q musl; then
            os="unknown-linux-musl"
        else
            os="unknown-linux-gnu"
        fi
    elif [[ "$os" == "windows" ]]; then
        os="pc-windows-msvc"
    fi

    echo "${arch}-${os}"
}

# Get latest release version from GitHub
get_latest_version() {
    local latest_url="https://api.github.com/repos/${GITHUB_REPO}/releases/latest"
    local version

    if command_exists curl; then
        version=$(curl -sL "$latest_url" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    elif command_exists wget; then
        version=$(wget -qO- "$latest_url" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    else
        print_error "Neither curl nor wget found. Please install one of them."
        exit 1
    fi

    if [[ -z "$version" ]]; then
        print_error "Failed to get latest version from GitHub"
        exit 1
    fi

    echo "$version"
}

# Download file
download_file() {
    local url="$1"
    local output="$2"

    if command_exists curl; then
        curl -fsSL "$url" -o "$output"
    elif command_exists wget; then
        wget -q "$url" -O "$output"
    else
        print_error "Neither curl nor wget found. Please install one of them."
        exit 1
    fi
}

# Verify checksum
verify_checksum() {
    local file="$1"
    local expected_checksum="$2"
    local actual_checksum

    if command_exists sha256sum; then
        actual_checksum=$(sha256sum "$file" | awk '{print $1}')
    elif command_exists shasum; then
        actual_checksum=$(shasum -a 256 "$file" | awk '{print $1}')
    else
        print_warning "Cannot verify checksum: sha256sum/shasum not found"
        return 0
    fi

    if [[ "$actual_checksum" != "$expected_checksum" ]]; then
        print_error "Checksum verification failed!"
        print_error "Expected: $expected_checksum"
        print_error "Actual: $actual_checksum"
        return 1
    fi

    print_success "Checksum verified successfully"
    return 0
}

# Check write permissions
check_permissions() {
    local dir="$1"

    # Create directory if it doesn't exist
    if [[ ! -d "$dir" ]]; then
        if ! mkdir -p "$dir" 2>/dev/null; then
            return 1
        fi
    fi

    # Check write permissions
    if [[ ! -w "$dir" ]]; then
        return 1
    fi

    return 0
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -d|--dir|--install-dir)
                INSTALL_DIR="$2"
                shift 2
                ;;
            -v|--version)
                VERSION="$2"
                shift 2
                ;;
            -h|--help)
                show_help
                exit 0
                ;;
            *)
                print_error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done
}

# Show help message
show_help() {
    cat << EOF
Claudelytics Installer

Usage: $0 [OPTIONS]

Options:
    -d, --dir, --install-dir DIR    Install directory (default: /usr/local/bin)
    -v, --version VERSION           Install specific version (default: latest)
    -h, --help                      Show this help message

Environment variables:
    INSTALL_DIR                     Install directory (same as --dir)

Examples:
    # Install to default location (/usr/local/bin)
    $0

    # Install to custom directory
    $0 --dir ~/.local/bin

    # Install specific version
    $0 --version v0.4.3

    # Using environment variable
    INSTALL_DIR=~/.local/bin $0
EOF
}

# Main installation function
main() {
    print_info "Claudelytics Installer"
    print_info "====================="
    echo

    # Parse arguments
    parse_args "$@"

    # Expand tilde in install directory
    INSTALL_DIR="${INSTALL_DIR/#\~/$HOME}"

    # Detect platform
    print_info "Detecting platform..."
    PLATFORM=$(detect_platform)
    print_success "Platform: $PLATFORM"

    # Get version
    if [[ -z "${VERSION:-}" ]]; then
        print_info "Getting latest version..."
        VERSION=$(get_latest_version)
    fi
    print_success "Version: $VERSION"

    # Check permissions
    print_info "Checking permissions for $INSTALL_DIR..."
    if ! check_permissions "$INSTALL_DIR"; then
        print_error "Cannot write to $INSTALL_DIR"
        print_info "Try one of the following:"
        print_info "  1. Run with sudo: sudo $0"
        print_info "  2. Choose a different directory: $0 --dir ~/.local/bin"
        exit 1
    fi

    # Construct download URLs
    BINARY_NAME="${PROGRAM_NAME}-${PLATFORM}"
    ARCHIVE_NAME="${BINARY_NAME}.tar.gz"
    DOWNLOAD_URL="https://github.com/${GITHUB_REPO}/releases/download/${VERSION}/${ARCHIVE_NAME}"
    CHECKSUM_URL="https://github.com/${GITHUB_REPO}/releases/download/${VERSION}/checksums.txt"

    # Create temporary directory
    TMP_DIR=$(mktemp -d)
    trap "rm -rf $TMP_DIR" EXIT

    # Download archive
    print_info "Downloading ${PROGRAM_NAME} ${VERSION}..."
    if ! download_file "$DOWNLOAD_URL" "$TMP_DIR/$ARCHIVE_NAME"; then
        print_error "Failed to download ${PROGRAM_NAME}"
        exit 1
    fi

    # Download and verify checksum
    print_info "Downloading checksums..."
    if download_file "$CHECKSUM_URL" "$TMP_DIR/checksums.txt" 2>/dev/null; then
        # Extract checksum for our file
        EXPECTED_CHECKSUM=$(grep "$ARCHIVE_NAME" "$TMP_DIR/checksums.txt" 2>/dev/null | awk '{print $1}')
        if [[ -n "$EXPECTED_CHECKSUM" ]]; then
            print_info "Verifying checksum..."
            if ! verify_checksum "$TMP_DIR/$ARCHIVE_NAME" "$EXPECTED_CHECKSUM"; then
                exit 1
            fi
        else
            print_warning "Checksum not found for $ARCHIVE_NAME, skipping verification"
        fi
    else
        print_warning "Checksums file not available, skipping verification"
    fi

    # Extract archive
    print_info "Extracting archive..."
    if ! tar -xzf "$TMP_DIR/$ARCHIVE_NAME" -C "$TMP_DIR"; then
        print_error "Failed to extract archive"
        exit 1
    fi

    # Find the binary (it should be named claudelytics)
    if [[ ! -f "$TMP_DIR/$PROGRAM_NAME" ]]; then
        print_error "Binary not found in archive"
        exit 1
    fi

    # Install binary
    print_info "Installing ${PROGRAM_NAME} to $INSTALL_DIR..."
    if ! mv "$TMP_DIR/$PROGRAM_NAME" "$INSTALL_DIR/$PROGRAM_NAME"; then
        print_error "Failed to install ${PROGRAM_NAME}"
        exit 1
    fi

    # Make executable
    chmod +x "$INSTALL_DIR/$PROGRAM_NAME"

    # Verify installation
    print_info "Verifying installation..."
    if ! "$INSTALL_DIR/$PROGRAM_NAME" --version >/dev/null 2>&1; then
        print_error "Installation verification failed"
        exit 1
    fi

    # Check if install directory is in PATH
    if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
        print_warning "$INSTALL_DIR is not in your PATH"
        print_info "Add the following to your shell configuration file:"
        print_info "  export PATH=\"$INSTALL_DIR:\$PATH\""
    fi

    print_success "Successfully installed ${PROGRAM_NAME} ${VERSION} to $INSTALL_DIR/$PROGRAM_NAME"
    echo
    print_info "Run '${PROGRAM_NAME} --help' to get started"
}

# Run main function
main "$@"