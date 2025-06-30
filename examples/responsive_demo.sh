#!/bin/bash
# Demo script for responsive table functionality

echo "=== Claudelytics Responsive Tables Demo ==="
echo
echo "This demo shows how tables automatically adjust to terminal width."
echo

# Function to run command in different terminal widths
demo_width() {
    local width=$1
    local title=$2
    
    echo "────────────────────────────────────────────────────────────"
    echo "Terminal Width: $width columns - $title"
    echo "────────────────────────────────────────────────────────────"
    
    # Simulate different terminal widths using stty (if available)
    if command -v stty >/dev/null 2>&1; then
        original_cols=$(stty -a | grep -oE 'columns [0-9]+' | cut -d' ' -f2)
        stty cols $width 2>/dev/null || true
    fi
    
    # Run claudelytics with responsive flag
    cargo run --quiet -- --responsive daily --since 20240101 --until 20240107 2>/dev/null || \
        claudelytics --responsive daily --since 20240101 --until 20240107
    
    # Restore original terminal width
    if command -v stty >/dev/null 2>&1 && [ -n "$original_cols" ]; then
        stty cols $original_cols 2>/dev/null || true
    fi
    
    echo
}

# Test different terminal widths
echo "1. ULTRA COMPACT MODE (50 columns)"
demo_width 50 "Minimal columns, abbreviated headers"

echo "2. COMPACT MODE (70 columns)"
demo_width 70 "Core columns only"

echo "3. NORMAL MODE (100 columns)"
demo_width 100 "Standard column set"

echo "4. WIDE MODE (140 columns)"
demo_width 140 "Extended columns with efficiency metrics"

echo "5. FULL MODE (180 columns)"
demo_width 180 "All columns visible"

echo
echo "=== Demo Complete ==="
echo
echo "Features demonstrated:"
echo "✓ Auto-adjusting column widths based on terminal size"
echo "✓ Smart column hiding/showing based on priority"
echo "✓ Abbreviated headers in compact mode"
echo "✓ Merged I/O columns in ultra-compact mode"
echo "✓ Consistent readability at all terminal sizes"