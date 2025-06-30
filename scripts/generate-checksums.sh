#!/usr/bin/env bash
# Script to generate checksums for manual testing
# This mimics what the GitHub Actions workflow will do

set -euo pipefail

echo "Generating test checksums..."

# Create test directory
mkdir -p test-release

# Create dummy binaries for testing
echo "dummy binary content" > test-release/claudelytics
chmod +x test-release/claudelytics

# Create archives
tar -czf test-release/claudelytics-x86_64-unknown-linux-gnu.tar.gz -C test-release claudelytics
tar -czf test-release/claudelytics-x86_64-apple-darwin.tar.gz -C test-release claudelytics
tar -czf test-release/claudelytics-aarch64-apple-darwin.tar.gz -C test-release claudelytics

# Generate checksums
cd test-release
for file in *.tar.gz; do
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$file"
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$file"
    fi
done > checksums.txt
cd ..

echo "Test checksums generated in test-release/checksums.txt:"
cat test-release/checksums.txt

echo
echo "To test the install script with local files:"
echo "1. Start a local web server: python3 -m http.server 8000 --directory test-release"
echo "2. Modify install.sh to use http://localhost:8000 instead of GitHub"

# Clean up
rm -rf test-release