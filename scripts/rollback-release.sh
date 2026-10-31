#!/usr/bin/env bash
# Pulls a bad GitHub release and points users back at a good one.
# Prints what it would do unless --yes is given.
#
#   scripts/rollback-release.sh --bad v0.3.0 --good v0.2.0 [--yes]
set -euo pipefail

bad="" good="" yes=false
while [ $# -gt 0 ]; do
  case "$1" in
    --bad) bad="$2"; shift 2 ;;
    --good) good="$2"; shift 2 ;;
    --yes) yes=true; shift ;;
    *) echo "Unknown option: $1" >&2; exit 2 ;;
  esac
done
if [ -z "$bad" ] || [ -z "$good" ] || [ "$bad" = "$good" ]; then
  echo "Usage: $0 --bad vX.Y.Z --good vX.Y.Z [--yes]" >&2
  exit 2
fi

# Both must exist before anything changes.
gh release view "$bad" --json tagName > /dev/null
gh release view "$good" --json tagName > /dev/null
if [ "$(gh release view "$good" --json isPrerelease --jq .isPrerelease)" = true ]; then
  echo "$good is a pre-release, so it can't be the release users fall back to." >&2
  exit 1
fi

notes=$(mktemp)
trap 'rm -f "$notes"' EXIT
{
  echo "**Withdrawn: use [$good](../../releases/tag/$good) instead.**"
  echo
  gh release view "$bad" --json body --jq .body
} > "$notes"

echo "Will mark $bad as a withdrawn pre-release, and make $good the latest release."
if [ "$yes" != true ]; then
  echo "Nothing changed. Add --yes to do it."
  exit 0
fi
gh release edit "$bad" --prerelease --latest=false --notes-file "$notes" > /dev/null
gh release edit "$good" --latest > /dev/null
echo "Done on GitHub."
echo "If $bad is on Google Play, also run: scripts/play-console-automation.py --creds KEY --track production halt"
