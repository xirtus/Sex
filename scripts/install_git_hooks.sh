#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if [[ ! -d .githooks ]]; then
  echo "missing .githooks directory"
  exit 1
fi

git config core.hooksPath .githooks
chmod +x .githooks/pre-commit

echo "Git hooks installed. pre-commit now enforces scripts/sexos_pipeline.sh"
