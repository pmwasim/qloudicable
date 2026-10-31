# Releasing QloudiCable

## Channels

| Channel | Tag | Who gets it | Shown in the app as |
| --- | --- | --- | --- |
| dev | none | Local builds | `0.2.0 (dev)` |
| beta | `v0.2.0-beta.1` | GitHub pre-release, Play internal or beta track | `0.2.0 (beta)` |
| stable | `v0.2.0` | GitHub latest, Play production | `0.2.0` |

The app has no runtime switches for these. The channel is fixed when CI builds it.

## Checks before anything ships

1. **Every push:** `ci.yml` runs tests and lints on Ubuntu, Windows and macOS.
2. **Every tag:** `release.yml` checks the tag matches `Cargo.toml`, tests and builds every platform, runs the CLI on each desktop OS, and creates a **draft** release.
3. **Before publishing a draft:** a person works through [QA-CHECKLIST.md](QA-CHECKLIST.md). Automated tests use simulated data, so they can't show that a real cable reads correctly.

To test the build pipeline without releasing anything, run `Release` from the Actions tab on any branch.

## Cutting a release

1. Set the version in `Cargo.toml` and `app/src-tauri/tauri.conf.json`, and commit.
2. Tag and push, using `vX.Y.Z-beta.N` first:

   ```bash
   git tag v0.2.0-beta.1
   git push origin v0.2.0-beta.1
   ```

3. Check the draft on the Releases page, run the checklist, then publish it.
4. When the beta holds up, tag `vX.Y.Z` the same way.

## Reaching users gradually

QloudiCable collects no usage data, so it can't run A/B experiments or tell how a variant performs on users' devices. Reducing risk comes from releasing in stages instead:

1. GitHub pre-release and Play internal track.
2. Play beta track.
3. Play production at a small fraction, watching crash and ANR rates in Play Console, which Google gathers without code in the app.

```bash
scripts/play-console-automation.py --creds KEY --track production release --aab FILE --fraction 0.1
scripts/play-console-automation.py --creds KEY --track production rollout --fraction 0.5
scripts/play-console-automation.py --creds KEY --track production complete
```

Run the Android workflow from the Actions tab to build and upload a bundle with CI. It needs these repository secrets: `ANDROID_RELEASE_KEYSTORE_BASE64`, `ANDROID_KEY_PROPERTIES`, `PLAY_CONSOLE_SERVICE_ACCOUNT_JSON`.

## Rolling back

**GitHub:** marks the bad release as withdrawn and makes the good one latest. It prints the plan first, and only acts with `--yes`.

```bash
scripts/rollback-release.sh --bad v0.3.0 --good v0.2.0 --yes
```

**Google Play:** stop the rollout first.

```bash
scripts/play-console-automation.py --creds KEY --track production halt
```

Play can't move users back to an older version. Anyone who already updated stays on it until you ship a fix with a higher version code, so the way out is a new release with the bad change reverted.

**Code:** revert the commit on `main` and release again. Don't move or reuse a published tag.

## Risks and what covers them

| Risk | Covered by |
| --- | --- |
| A change breaks one OS | CI on all three desktop OSes; every tag builds all four platforms |
| A bad build goes out automatically | Tags only create drafts; a person publishes |
| A wrong reading on real hardware | QA checklist before publishing |
| A crash on some Android phones | Staged rollout, then halt |
| Lost Android upload key | Keep a backup outside this machine and outside this repository |
