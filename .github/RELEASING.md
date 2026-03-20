# Releasing LadderMD

## How to create a release

1. Update version numbers in:
   - `crates/laddermd-core/Cargo.toml`
   - `crates/laddermd-cli/Cargo.toml`
   - `crates/laddermd-web/Cargo.toml`
   - `crates/laddermd-mcp/Cargo.toml`
   - `crates/laddermd-desktop/src-tauri/Cargo.toml`
   - `crates/laddermd-desktop/src-tauri/tauri.conf.json`

2. Commit the version bump:
   ```bash
   git add -A
   git commit -m "chore: bump version to v0.2.0"
   ```

3. Create and push a tag:
   ```bash
   git tag v0.2.0
   git push origin main --tags
   ```

4. The `release.yml` workflow will automatically:
   - Create a GitHub Release with auto-generated release notes
   - Build CLI binaries for Linux (x86_64, aarch64), macOS (x86_64, aarch64), Windows (x86_64)
   - Build MCP server binaries for Linux, macOS, Windows
   - Build Tauri desktop app installers:
     - macOS: `.dmg` (x86_64 + aarch64)
     - Linux: `.deb`, `.AppImage`
     - Windows: `.msi`, `.exe`
   - Attach all artifacts to the release

## Release artifacts

| Artifact | Platforms | Format |
|----------|-----------|--------|
| CLI (`laddermd-cli`) | Linux, macOS, Windows | `.tar.gz` / `.zip` |
| MCP Server (`laddermd-mcp`) | Linux, macOS, Windows | `.tar.gz` / `.zip` |
| Desktop App | macOS | `.dmg` |
| Desktop App | Linux | `.deb`, `.AppImage` |
| Desktop App | Windows | `.msi`, `.exe` |

## Pre-release

To create a pre-release, use a tag with a hyphen (e.g., `v0.2.0-beta.1`).
The release will be automatically marked as a pre-release.
