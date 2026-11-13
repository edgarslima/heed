#!/usr/bin/env bash
# setup-environment.sh — prepares macOS for Tauri 2 + Rust + Swift + TypeScript development.
# Safe to run more than once: it skips anything already installed.
# Usage: bash setup-environment.sh
set -euo pipefail

info() { printf '\n\033[1;34m==> %s\033[0m\n' "$*"; }
ok()   { printf '    \033[32m✔ %s\033[0m\n' "$*"; }
fail() { printf '    \033[31m✘ %s\033[0m\n' "$*"; exit 1; }

# 1. System --------------------------------------------------------------
info "Checking system"
[[ "$(uname -s)" == "Darwin" ]] || fail "This script is for macOS only."
ok "macOS $(sw_vers -productVersion) ($(uname -m))"

# 2. Xcode / Swift --------------------------------------------------------
info "Checking Xcode and Swift"
xcode-select -p >/dev/null 2>&1 || fail "Xcode not found. Install it from the App Store and run the script again."
if ! xcodebuild -license check >/dev/null 2>&1; then
  fail "Xcode license is not accepted. Run: sudo xcodebuild -license accept"
fi
ok "$(xcodebuild -version | head -1)"
ok "$(swift --version 2>&1 | grep -o 'Swift version [0-9.]*')"

# 3. Homebrew -------------------------------------------------------------
info "Checking Homebrew"
command -v brew >/dev/null 2>&1 || fail "Homebrew not found. Install it at https://brew.sh"
ok "$(brew --version | head -1)"

# 4. Rust (via the official rustup) --------------------------------------
info "Installing/updating Rust"
if ! command -v rustup >/dev/null 2>&1 && [[ ! -x "$HOME/.cargo/bin/rustup" ]]; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --default-toolchain stable --profile default
fi
# shellcheck disable=SC1091
source "$HOME/.cargo/env"
rustup update stable
# Targets to generate a universal macOS app (Apple Silicon + Intel)
rustup target add aarch64-apple-darwin x86_64-apple-darwin
# Coverage tooling required by the coding standard (cargo-llvm-cov gate)
rustup component add llvm-tools-preview
if ! cargo llvm-cov --version >/dev/null 2>&1; then
  cargo install cargo-llvm-cov --locked
fi
ok "$(rustc --version)"
ok "$(cargo --version)"
ok "$(cargo llvm-cov --version)"

# 5. Tauri CLI (Rust) ----------------------------------------------------
info "Installing Tauri CLI v2 (the first build may take a few minutes)"
if cargo tauri --version 2>/dev/null | grep -q '^tauri-cli 2'; then
  ok "$(cargo tauri --version) already installed"
else
  cargo install tauri-cli --version '^2' --locked
  ok "$(cargo tauri --version)"
fi

# 6. Node / TypeScript ---------------------------------------------------
info "Checking Node.js"
if ! command -v node >/dev/null 2>&1; then
  brew install node@24
  # node@24 is keg-only, so Homebrew does not link it into PATH.
  export PATH="$(brew --prefix node@24)/bin:$PATH"
  printf '    Add to your shell profile: export PATH="%s/bin:$PATH"\n' "$(brew --prefix node@24)"
fi
ok "Node $(node --version) / npm $(npm --version)"
command -v pnpm >/dev/null 2>&1 && ok "pnpm $(pnpm --version)"

# 7. Audio inspection --------------------------------------------------
info "Checking ffprobe (used by the technical tests)"
command -v ffprobe >/dev/null 2>&1 || brew install ffmpeg
ok "$(ffprobe -version | head -1)"

# 8. Summary ---------------------------------------------------------------
info "Environment ready!"
cat <<'EOF'
    Open a new terminal (or run: source "$HOME/.cargo/env") so cargo is on PATH.

    Create a new project:
      npm create tauri-app@latest      # choose TypeScript + your preferred UI framework
      cd <project> && npm install
      npm run tauri dev                # runs in development mode
      npm run tauri build              # generates the .app / .dmg

    Universal build (Apple Silicon + Intel):
      npm run tauri build -- --target universal-apple-darwin

    Windows and Linux: generate the installers via GitHub Actions (tauri-apps/tauri-action).
EOF
