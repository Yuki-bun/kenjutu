#!/bin/bash
set -euo pipefail

if command -v jj &>/dev/null; then
  echo "jujutsu is already installed: $(jj --version)"
  exit 0
fi

echo "Installing jujutsu..."

# Download pre-built binary (much faster than cargo install)
JJ_VERSION="0.38.2"
ARCHIVE="jj-v${JJ_VERSION}-x86_64-unknown-linux-musl.tar.gz"
URL="https://github.com/jj-vcs/jj/releases/download/v${JJ_VERSION}/${ARCHIVE}"

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

curl -fsSL "$URL" -o "$TMPDIR/$ARCHIVE"
tar -xzf "$TMPDIR/$ARCHIVE" -C "$TMPDIR"

# Install to ~/.local/bin (usually on PATH in remote environments)
mkdir -p "$HOME/.local/bin"
mv "$TMPDIR/jj" "$HOME/.local/bin/jj"
chmod +x "$HOME/.local/bin/jj"

# Ensure ~/.local/bin is on PATH for the session
if [ -n "${CLAUDE_ENV_FILE:-}" ]; then
  echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$CLAUDE_ENV_FILE"
fi

echo "jujutsu installed: $("$HOME/.local/bin/jj" --version)"

jj git init
