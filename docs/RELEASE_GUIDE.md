# Release Guide

Steps to ship a new version of SerialBridge.

## Prerequisites (one-time)

1. Install Xcode Command Line Tools: `xcode-select --install`
2. Install Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
3. Install Node.js 20+: https://nodejs.org
4. Add Rust targets for universal binary:
   ```bash
   rustup target add aarch64-apple-darwin x86_64-apple-darwin
   ```

## Build unsigned DMG (fastest, for hackathon submission)

```bash
# From project root
npm ci
npm run tauri build
```

Output location (version from `tauri.conf.json`, currently 0.2.2):
```
src-tauri/target/release/bundle/dmg/SerialBridge_0.2.2_aarch64.dmg   # Apple Silicon
src-tauri/target/release/bundle/dmg/SerialBridge_0.2.2_x64.dmg       # Intel
src-tauri/target/release/bundle/macos/SerialBridge.app               # .app bundle
```

## Build universal DMG (Intel + ARM in one)

```bash
npm run tauri build -- --target universal-apple-darwin
# Output: src-tauri/target/universal-apple-darwin/release/bundle/dmg/SerialBridge_0.2.2_universal.dmg
```

## Unsigned DMG — installation note for users

Because the DMG isn't signed with an Apple Developer ID, macOS will block it by default. Add this to the GitHub release notes:

> **macOS quarantine bypass**
>
> After downloading and dragging SerialBridge to `/Applications`:
>
> 1. Try to open — macOS will say "cannot be opened because Apple cannot check it for malicious software"
> 2. Open **System Settings → Privacy & Security**
> 3. Scroll down, click **"Open Anyway"** next to the SerialBridge blocked notice
> 4. Confirm → SerialBridge launches
>
> Alternative via Terminal: `xattr -d com.apple.quarantine /Applications/SerialBridge.app`

## Signed DMG (proper release)

Requires Apple Developer Program membership ($99/yr). See:
https://v2.tauri.app/distribute/sign/macos/

## GitHub Release

Push a `v*` tag; `.github/workflows/release.yml` builds artifacts. Release name: `SerialBridge <tag>`.

## Version bump

Update together:
- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`
- local package entry in `src-tauri/Cargo.lock` (via `cargo check` in `src-tauri`)

## Windows build

```bash
npm run tauri build -- --target x86_64-pc-windows-msvc
```

Output: `src-tauri/target/release/bundle/msi/SerialBridge_0.2.2_x64_en-US.msi` (version may vary)

## Linux build

```bash
npm run tauri build
# Output: src-tauri/target/release/bundle/appimage/serialbridge_0.2.2_amd64.AppImage
#         src-tauri/target/release/bundle/deb/serialbridge_0.2.2_amd64.deb
```

## Hackathon submission checklist

- [ ] DMG built successfully (`npm run tauri build`)
- [ ] Demo script in [JUDGE_DEMO.md](./JUDGE_DEMO.md) recorded from `npm run tauri dev`
- [ ] Meta Model API key configured in Settings (host-held, not in repo)
- [ ] `.env` with secrets is NOT committed
- [ ] `LICENSE` file present
