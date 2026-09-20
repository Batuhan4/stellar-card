#!/usr/bin/env bash
#
# Install the stellar-card-agent skill for Codex (and optionally Claude Code).
#
# Usage:
#   ./scripts/install-skill.sh            # install into $CODEX_HOME/skills
#   ./scripts/install-skill.sh --claude   # also install into ~/.claude/skills
#   ./scripts/install-skill.sh --all      # same as --claude
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
SKILL_SRC="${REPO_ROOT}/.agents/skills/stellar-card-agent"

CODEX_HOME="${CODEX_HOME:-${HOME}/.codex}"
CODEX_SKILLS_DIR="${CODEX_HOME}/skills"
CLAUDE_SKILLS_DIR="${HOME}/.claude/skills"

usage() {
  cat <<'EOF'
Install the stellar-card-agent skill.

Usage:
  install-skill.sh [--claude|--all]

Options:
  --claude, --all   Also install for Claude Code into ~/.claude/skills
  -h, --help        Show this help

Environment:
  CODEX_HOME        Codex home directory (default: ~/.codex)
  HOME              Used to resolve ~/.codex and ~/.claude
EOF
}

install_skill() {
  local skills_dir="$1"
  local label="$2"
  mkdir -p "${skills_dir}"
  rm -rf "${skills_dir}/stellar-card-agent"
  cp -R "${SKILL_SRC}" "${skills_dir}/stellar-card-agent"
  printf 'Installed stellar-card-agent into %s (%s)\n' \
    "${skills_dir}/stellar-card-agent" "${label}"
}

install_claude=false
while [[ $# -gt 0 ]]; do
  case "$1" in
    --claude|--all)
      install_claude=true
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      printf 'error: unknown option: %s\n\n' "$1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ ! -f "${SKILL_SRC}/SKILL.md" ]]; then
  printf 'error: skill source not found at %s\n' "${SKILL_SRC}" >&2
  exit 1
fi

install_skill "${CODEX_SKILLS_DIR}" "Codex"
if [[ "${install_claude}" == true ]]; then
  install_skill "${CLAUDE_SKILLS_DIR}" "Claude Code"
fi

printf 'Done. Restart the agent session so it picks up the skill.\n'
