# Release v0.3.2

## 🐛 Bug Fixes

- **Fixed dead code warning**: Removed unused functions `display_model_breakdown_json` and `display_model_breakdown_as_full_table` from display.rs
- **Improved error handling**: Replaced a potentially unsafe `unwrap()` call in parser.rs with proper error handling
- **Fixed model breakdown display**: The `--by-model` flag now correctly supports both `CLAUDELYTICS_TABLE_FORMAT` and `CLAUDELYTICS_DISPLAY_FORMAT` environment variables for table display

## 🧹 Code Cleanup

- Removed orphaned code blocks in display.rs that were causing compilation errors
- Removed FIX_SUMMARY.md as it documented a fix that has been properly implemented
- Ensured all code passes `cargo fmt` and `cargo clippy` checks with no warnings

## 📚 Documentation Updates

- Updated README.md to clarify that both `CLAUDELYTICS_TABLE_FORMAT` and `CLAUDELYTICS_DISPLAY_FORMAT=table` work for table display
- Updated CLAUDE.md with the alternative table format command

## ✅ Quality Assurance

- All 30 tests pass successfully
- No clippy warnings
- Code properly formatted with cargo fmt
- CI/CD checks pass without issues

This is a maintenance release focused on code quality and fixing minor issues. No breaking changes or new features.