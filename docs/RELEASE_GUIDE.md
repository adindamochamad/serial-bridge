# Release Guide

Steps to ship a new version of OmniBridge.

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
npm install
npm run tauri build
```

Output location:
```
src-tauri/target/release/bundle/dmg/OmniBridge_0.1.0_aarch64.dmg   # Apple Silicon
src-tauri/target/release/bundle/dmg/OmniBridge_0.1.0_x64.dmg       # Intel
src-tauri/target/release/bundle/macos/OmniBridge.app               # .app bundle
```

## Build universal DMG (Intel + ARM in one)

```bash
npm run tauri build -- --target universal-apple-darwin
# Output: src-tauri/target/universal-apple-darwin/release/bundle/dmg/OmniBridge_0.1.0_universal.dmg
```

## Unsigned DMG — installation note for users

Because the DMG isn't signed with an Apple Developer ID, macOS will block it by default. Add this to the GitHub release notes:

> **macOS quarantine bypass**
>
> After downloading and dragging OmniBridge to `/Applications`:
>
> 1. Try to open — macOS will say "cannot be opened because Apple cannot check it for malicious software"
> 2. Open **System Settings → Privacy & Security**
> 3. Scroll down, click **"Open Anyway"** next to the OmniBridge blocked notice
> 4. Confirm → OmniBridge launches
>
> Alternative via Terminal: `xattr -d com.apple.quarantine /Applications/OmniBridge.app`

## Signed DMG (proper release)

Requires Apple Developer Program membership ($99/yr). See:
https://v2.tauri.app/distribute/sign/macos/

Add to `tauri.conf.json`:
```json
"macOS": {
  "signingIdentity": "Developer ID Application: YOUR NAME (TEAMID)",
  "providerShortName": "TEAMID"
}
```

## GitHub Release

1. Tag the commit:
   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```

2. Create release via `gh` CLI:
   ```bash
   gh release create v0.1.0 \
     --title "OmniBridge v0.1.0 — First Release" \
     --notes-file docs/RELEASE_NOTES_0.1.0.md \
     src-tauri/target/release/bundle/dmg/*.dmg
   ```

   Or via web: GitHub → Releases → Draft new release → attach DMG files

3. Update the `README.md` Releases link

## Version bump

Update in 3 places:
- `package.json` → `"version": "0.1.1"`
- `src-tauri/Cargo.toml` → `version = "0.1.1"`
- `src-tauri/tauri.conf.json` → `"version": "0.1.1"`

## Windows build

```bash
npm run tauri build -- --target x86_64-pc-windows-msvc
# Requires Windows machine or cross-compilation setup
```

Output: `src-tauri/target/release/bundle/msi/OmniBridge_0.1.0_x64_en-US.msi`

## Linux build

```bash
npm run tauri build
# Output: src-tauri/target/release/bundle/appimage/omnibridge_0.1.0_amd64.AppImage
#         src-tauri/target/release/bundle/deb/omnibridge_0.1.0_amd64.deb
```

## Hackathon submission checklist

- [ ] DMG built successfully (`npm run tauri build`)
- [ ] DMG tested on a separate user account (no dev dependencies)
- [ ] Video recorded and exported ≤ 30MB
- [ ] README screenshots added (if any)
- [ ] GitHub repo is public with clear README
- [ ] Release created with DMG attached
- [ ] Submission form filled with repo URL + video URL
- [ ] `.env` with API key is NOT committed (verify: `git ls-files | grep .env`)
- [ ] `LICENSE` file present
