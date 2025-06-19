# Development Environment Setup

Quick setup workflow for claudelytics development.

## Command: /project:dev-setup

### Usage
```bash
/project:dev-setup                 # Full development setup
/project:dev-setup tools           # Install dev tools only
/project:dev-setup deps            # Update dependencies
/project:dev-setup targets         # Setup cross-compilation
/project:dev-setup hooks           # Setup git hooks
```

### Setup Process

#### Step 1: Rust Toolchain Setup
```bash
# Ensure latest stable Rust
rustup update stable
rustup default stable

# Install required components
rustup component add rustfmt clippy

# Install cargo extensions
cargo install cargo-watch cargo-edit cargo-outdated
cargo install cargo-audit cargo-tarpaulin
```

#### Step 2: Cross-Compilation Targets
```bash
# Add all supported targets
rustup target add x86_64-unknown-linux-gnu
rustup target add x86_64-unknown-linux-musl
rustup target add x86_64-pc-windows-msvc
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin

# Verify targets
rustup target list --installed
```

#### Step 3: Development Dependencies
```bash
# Update dependencies
cargo update

# Check for outdated deps
cargo outdated

# Audit for security vulnerabilities
cargo audit
```

#### Step 4: Git Hooks Setup
Create `.git/hooks/pre-commit`:
```bash
#!/usr/bin/env bash
set -euo pipefail

echo "Running pre-commit checks..."

# Format check
if ! cargo fmt -- --check; then
    echo "❌ Formatting issues found. Run 'cargo fmt' to fix."
    exit 1
fi

# Clippy check
if ! cargo clippy -- -D warnings; then
    echo "❌ Clippy warnings found. Please fix before committing."
    exit 1
fi

# Test check
if ! cargo test --quiet; then
    echo "❌ Tests failing. Please fix before committing."
    exit 1
fi

echo "✅ All pre-commit checks passed!"
```

Make it executable:
```bash
chmod +x .git/hooks/pre-commit
```

#### Step 5: Development Tools
```bash
# Install additional tools
cargo install tokei              # Code statistics
cargo install ripgrep            # Fast grep
cargo install cargo-expand       # Macro expansion
cargo install cargo-criterion    # Benchmarking

# Platform-specific tools
# macOS:
brew install peco               # For interactive mode testing

# Linux:
# sudo apt-get install peco     # Debian/Ubuntu
# sudo pacman -S peco          # Arch
```

#### Step 6: IDE/Editor Setup

**VS Code Extensions**:
```json
{
  "recommendations": [
    "rust-lang.rust-analyzer",
    "tamasfe.even-better-toml",
    "serayuzgur.crates",
    "vadimcn.vscode-lldb"
  ]
}
```

**Settings for VS Code**:
```json
{
  "rust-analyzer.checkOnSave.command": "clippy",
  "rust-analyzer.cargo.features": "all",
  "[rust]": {
    "editor.formatOnSave": true
  }
}
```

#### Step 7: Test Data Setup
```bash
# Create test data directory
mkdir -p test_data/claude/projects/test_project

# Generate sample JSONL files
cat > test_data/sample.jsonl << 'EOF'
{"timestamp":"2024-01-01T10:00:00Z","message":{"usage":{"input_tokens":1000,"output_tokens":2000}},"costUSD":0.15}
{"timestamp":"2024-01-01T11:00:00Z","message":{"usage":{"input_tokens":500,"output_tokens":1500}},"costUSD":0.10}
EOF

# Set test environment
export CLAUDELYTICS_TEST_PATH="./test_data"
```

### Quick Development Commands

```bash
# Watch for changes and auto-run tests
cargo watch -x test

# Run with debug output
RUST_LOG=debug cargo run -- daily

# Quick benchmark
cargo bench --bench parser_bench

# Generate documentation
cargo doc --no-deps --open

# Expand macros for debugging
cargo expand models
```

### Environment Variables
```bash
# Development settings
export RUST_BACKTRACE=1
export RUST_LOG=claudelytics=debug
export CLAUDELYTICS_TEST_MODE=1

# Optional: Custom Claude path for testing
export CLAUDE_HOME="./test_data/claude"
```

### Troubleshooting Setup

**Common Issues:**
1. **Cross-compilation fails**
   ```bash
   # Install required linkers
   # macOS → Linux:
   brew install messense/macos-cross-toolchains/x86_64-unknown-linux-gnu
   ```

2. **Tests fail on fresh clone**
   ```bash
   # Ensure test data exists
   ./scripts/setup-test-data.sh
   ```

3. **Git hooks not running**
   ```bash
   # Check hook permissions
   ls -la .git/hooks/pre-commit
   chmod +x .git/hooks/pre-commit
   ```

### Success Checklist
- ✅ Latest Rust toolchain installed
- ✅ All targets added
- ✅ Development tools installed
- ✅ Git hooks configured
- ✅ IDE properly configured
- ✅ Test data available
- ✅ First build successful

### Time Savings
Manual setup: ~45-60 minutes
Automated setup: ~10 minutes
**Time saved: 35+ minutes for new developers**

### Verification
```bash
# Run full verification
cargo fmt --check && \
cargo clippy -- -D warnings && \
cargo test && \
cargo build --release && \
echo "✅ Development environment ready!"
```