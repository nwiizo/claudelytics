# Responsive Tables in Claudelytics

The responsive tables feature automatically adjusts table layouts based on terminal width, ensuring optimal readability across different screen sizes and terminal configurations.

## Overview

Responsive tables intelligently adapt to terminal width by:
- Auto-adjusting column widths
- Hiding/showing columns based on priority
- Using abbreviated headers in compact modes
- Merging related columns when space is limited
- Maintaining readability at all terminal sizes

## Usage

Add the `--responsive` flag to any command that displays tables:

```bash
# Daily report with responsive tables
claudelytics --responsive daily

# Session report with responsive tables  
claudelytics --responsive session

# Billing blocks with responsive tables
claudelytics --responsive billing-blocks

# Combine with other flags
claudelytics --responsive --today daily
claudelytics --responsive --since 20240101 session
```

## Terminal Width Modes

The system automatically detects terminal width and adjusts accordingly:

### Ultra Compact Mode (< 60 columns)
- Shows only essential columns: Date/Time and Cost
- Uses shortest possible headers
- Merges Input/Output tokens into single "I/O" column
- Ideal for mobile terminals or split panes

### Compact Mode (60-80 columns)
- Adds Total Tokens column
- Still uses abbreviated headers
- May merge related columns to save space
- Good for narrow terminal windows

### Normal Mode (80-120 columns)
- Standard column set with full headers
- Shows Input and Output tokens separately
- Includes basic metrics
- Default for most terminal applications

### Wide Mode (120-160 columns)
- Adds efficiency metrics (tokens per dollar)
- Shows O/I ratio for daily reports
- Includes cache token information
- Better for data analysis

### Full Mode (> 160 columns)
- All columns visible
- Detailed metrics and ratios
- No column merging
- Maximum information density

## Column Priority System

Columns are displayed based on priority (1 = highest):

1. **Priority 1** (Always shown):
   - Date/Time
   - Cost

2. **Priority 2**:
   - Total Tokens

3. **Priority 3**:
   - Input Tokens
   - Output Tokens

4. **Priority 4**:
   - Cache Tokens
   - Last Activity (sessions)

5. **Priority 5**:
   - Efficiency metrics
   - Ratios

## Features

### Smart Column Merging
In compact modes, related columns can be merged:
- Input/Output tokens → "I/O: 1.2K/3.4K"
- Cache creation/read → "Cache: 500"

### Abbreviated Headers
Compact modes use shorter headers:
- "Cost (USD)" → "Cost"
- "Total Tokens" → "Tokens"
- "Efficiency" → "Eff"

### Responsive Summary Cards
Summary sections also adapt to terminal width:
- Ultra compact: Basic cost and token count
- Compact: Add I/O breakdown
- Normal+: Full metrics with efficiency calculations

### Consistent Formatting
- Numbers use smart formatting (1.2K, 3.4M)
- Currency respects precision ($0.1234 vs $123.45)
- Colors remain consistent across all modes

## Examples

### Daily Report - Different Widths

**Ultra Compact (50 cols)**:
```
Date       Cost    Tokens
---------- ------- -------
2024-01-07 $0.1234   1.2K
2024-01-06 $0.5678   5.6K
Total      $0.6912   6.8K
```

**Normal (100 cols)**:
```
Date       Cost (USD)  Total Tokens  Input    Output   O/I Ratio
---------- ----------- ------------- -------- -------- ---------
2024-01-07 $0.1234     1,234         500      734      1.5:1
2024-01-06 $0.5678     5,678         2,000    3,678    1.8:1
Total      $0.6912     6,912         2,500    4,412    1.8:1
```

**Wide (140 cols)**:
```
Date       Cost (USD)  Total Tokens  Input    Output   Cache    O/I Ratio  Efficiency   Cache Hit
---------- ----------- ------------- -------- -------- -------- --------- ------------ ---------
2024-01-07 $0.1234     1,234         500      734      100      1.5:1     10,000 tok/$ 16.7%
2024-01-06 $0.5678     5,678         2,000    3,678    500      1.8:1     10,003 tok/$ 20.0%
Total      $0.6912     6,912         2,500    4,412    600      1.8:1     10,002 tok/$ 19.4%
```

## Best Practices

1. **Let it auto-detect**: The system automatically chooses the best layout
2. **Resize to see more**: Expand your terminal to see additional columns
3. **Use with `--json`**: For scripting, JSON output remains consistent
4. **Combine with filters**: Responsive tables work with all other flags

## Technical Details

The responsive table system is implemented in `src/responsive_tables.rs` and provides:
- `ResponsiveTable` struct for building adaptive tables
- `TableMode` enum for different display modes
- `TableColumn` definitions with priorities
- Column visibility algorithms
- Smart formatting functions

## Integration

The responsive tables integrate seamlessly with:
- Daily reports (`claudelytics daily`)
- Session reports (`claudelytics session`)
- Billing blocks (`claudelytics billing-blocks`)
- Monthly reports (coming soon)

## Future Enhancements

Planned improvements:
- User-configurable column priorities
- Custom width breakpoints
- Saved layout preferences
- Export responsive layouts to HTML