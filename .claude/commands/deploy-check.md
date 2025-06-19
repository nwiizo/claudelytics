# Deploy/Release Check

Pre-release verification workflow for claudelytics.

## Command: /project:deploy-check

### Usage
```bash
/project:deploy-check              # Full pre-release check
/project:deploy-check security     # Security scan only
/project:deploy-check perf         # Performance check only
/project:deploy-check docs         # Documentation verification
/project:deploy-check release      # Release preparation
```

### Deployment Checklist

#### Step 1: Version & Changelog Update
```bash
# Check current version
grep version Cargo.toml

# Update version if needed
# Update CHANGELOG.md with release notes
```

#### Step 2: Security Audit (1 task)
```bash
cargo audit
cargo outdated --exit-code 1
```

#### Step 3: Performance Verification (2 parallel tasks)
```bash
# Build size check
cargo build --release
ls -lh target/release/claudelytics

# Benchmark critical paths
cargo bench
```

#### Step 4: Cross-Platform Build Test (5 parallel tasks)
```bash
# Test all target platforms
cargo build --release --target x86_64-unknown-linux-gnu
cargo build --release --target x86_64-unknown-linux-musl  
cargo build --release --target x86_64-pc-windows-msvc
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin
```

#### Step 5: Documentation Verification
- Check README.md is up to date
- Verify INSTALL.md instructions work
- Ensure all new features are documented
- Generate and review API docs: `cargo doc --no-deps`

#### Step 6: Release Assets Preparation
```bash
# Create release binaries
./scripts/build-release.sh  # If exists

# Generate checksums
shasum -a 256 target/release/claudelytics-*
```

#### Step 7: Final Checks
- [ ] All tests pass in release mode
- [ ] No security vulnerabilities
- [ ] Documentation complete
- [ ] Version bumped appropriately
- [ ] CHANGELOG.md updated
- [ ] Release binaries build successfully
- [ ] Homebrew formula prepared (if needed)

### Success Criteria
- ✅ Zero security vulnerabilities
- ✅ All platforms build successfully
- ✅ Performance benchmarks pass
- ✅ Documentation complete and accurate
- ✅ Release assets ready

### Time Savings
Manual execution: ~45-60 minutes
Automated pipeline: ~10-15 minutes
**Time saved: 35+ minutes per release**

### Example Output
```
🚀 Starting Deploy Check...
✅ Step 1: Version 0.3.3 ready
✅ Step 2: Security audit passed
✅ Step 3: Performance verified
   - Binary size: 12.3MB (acceptable)
   - Benchmarks: All within limits
✅ Step 4: Cross-platform builds successful
✅ Step 5: Documentation verified
✅ Step 6: Release assets prepared
✅ Step 7: All checks passed

🎉 Ready for deployment!
Total time: 12m 45s

Next steps:
1. git tag v0.3.3
2. git push origin v0.3.3
3. GitHub Actions will handle the rest
```