# P00 packaging evidence

**Purpose:** Records the basic packaged-window check for P00.7. **Audience:** Reviewers of P00. **Status:** Recorded 2026-11-13.

| Field | Value |
| --- | --- |
| Commit | `a8614f7` (branch `initial_dev`) |
| Host | macOS 27.0.1, arm64 (MacBook Pro bench) |
| Toolchain | rustc 1.100.0, tauri-cli 2.12.4 (npm), Node 24.21.0 |
| Command | `npm run tauri build -- --bundles app` |
| Artifact | `target/release/bundle/macos/Heed.app` (3.81 MiB) |
| Bundle metadata | `CFBundleIdentifier` `pro.edgarlima.heed`, `CFBundleName` `Heed`, version `0.1.0` |
| Signature | Ad hoc (no Developer ID; signing is a P09 release step) |
| SHA-256, zipped bundle (`ditto -c -k --keepParent`) | `9c01de20351a1149a03b7534cd7f8563817756c8717db3c96f11036cac96559a` |
| SHA-256, `Contents/MacOS/heed` | `46a6a09c5b6f7ae3e0c070fa1c2f4c03ccfba8e132cb7561a29d09248ea4f30a` |

## Result

- `open target/release/bundle/macos/Heed.app` started the app; System Events reported window 1 titled "Heed" at 560 × 600.
- `lsof -nP -a -i -p <pid>` listed no network sockets for the app process; the UI is served from bundled assets.
- Not covered: networking was not disabled system-wide, and the separate WebKit networking process was not inspected. The full offline check of an installed build belongs to P07.5 and P09.3.

The zip hash is specific to this build (archive timestamps vary); the binary hash identifies the executable.
