# In-game self-update, checksum-verified, never elevated

Comparable recomps either update manually (Ship of Harkinian, Dusklight) or check in-game and hand off to a separate builder (Hookshot). We instead self-update in-process: download the release asset, verify it against the same release's `SHA256SUMS.txt`, replace the running copy by rename (`.exe.old` on Windows, `$APPIMAGE` on Linux, whole `.app` bundle on macOS), and restart. We do this because the game is a single portable file per platform, so there is no builder to delegate to, and players should not have to re-download by hand.

## Considered Options

- **Signed releases (minisign)**: rejected for now; it protects against a compromised GitHub account but adds private-key management in CI. Checksums only protect against corrupt or truncated downloads.
- **Elevating to write protected install folders**: rejected; the game never requests admin rights. If its folder is not writable, or Linux is not running an AppImage, it opens the download page instead.

## Consequences

- The release asset names and `SHA256SUMS.txt` format are a contract with every shipped copy; changing them breaks self-update for older installs.
- Pre-releases are never offered, and there is no in-game rollback.
