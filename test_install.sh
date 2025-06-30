#!/usr/bin/env bash
set -euo pipefail

# Test script for install.sh

echo "Testing claudelytics install script..."
echo

# Test 1: Help option
echo "Test 1: Testing --help option"
./install.sh --help
echo

# Test 2: Show what would be installed (dry run)
echo "Test 2: Testing platform detection"
./install.sh --dir /tmp/test-claudelytics --version v0.4.3 || true
echo

# Clean up
rm -rf /tmp/test-claudelytics

echo "All tests completed!"