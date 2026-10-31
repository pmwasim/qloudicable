#!/usr/bin/env python3
"""
Google Play releases for QloudiCable, through the Play Developer API.

  release   Upload a bundle to a track, optionally to a fraction of users
  status    Show what each release on a track is doing
  rollout   Change the fraction of users on a staged rollout
  halt      Stop a staged rollout
  resume    Restart a halted rollout
  complete  Send a staged rollout to all users
  testers   Set the Google Groups allowed to test a track

Needs a Play service account key. See docs/RELEASING.md.
"""

import argparse
import os
import sys

PACKAGE_NAME = "com.qloudihub.qloudicable"
DEFAULT_TRACK = "internal"


def parse_fraction(value):
    """Play takes a staged fraction strictly between 0 and 1."""
    fraction = float(value)
    if not 0 < fraction < 1:
        raise argparse.ArgumentTypeError("fraction must be between 0 and 1, for example 0.1 for 10%")
    return fraction


def new_release(version_code, notes, fraction=None):
    release = {
        "versionCodes": [str(version_code)],
        "status": "inProgress" if fraction else "completed",
        "releaseNotes": [{"language": "en-US", "text": notes}],
    }
    if fraction:
        release["userFraction"] = fraction
    return release


def change_rollout(releases, action, fraction=None):
    """Returns the track's releases with the action applied to the staged one.

    Pure, so it can be tested without Google. Raises ValueError rather than
    guessing when no release is in the right state.
    """
    wanted = {
        "rollout": ("inProgress",),
        "halt": ("inProgress",),
        "resume": ("halted",),
        "complete": ("inProgress", "halted"),
    }[action]
    updated = [dict(r) for r in releases]
    staged = [r for r in updated if r.get("status") in wanted]
    if len(staged) != 1:
        raise ValueError(f"Expected one release in state {' or '.join(wanted)}, found {len(staged)}")
    release = staged[0]
    if action == "rollout":
        if fraction is None:
            raise ValueError("rollout needs a fraction")
        release["userFraction"] = fraction
    elif action == "halt":
        release["status"] = "halted"
    elif action == "resume":
        release["status"] = "inProgress"
    elif action == "complete":
        release["status"] = "completed"
        release.pop("userFraction", None)
    return updated


def get_service(credentials_path):
    try:
        from google.oauth2 import service_account
        from googleapiclient.discovery import build
    except ImportError:
        sys.exit("Missing packages. Run: pip install google-api-python-client google-auth")
    if not os.path.exists(credentials_path):
        sys.exit(f"Credentials file not found: {credentials_path}")
    scopes = ["https://www.googleapis.com/auth/androidpublisher"]
    creds = service_account.Credentials.from_service_account_file(credentials_path, scopes=scopes)
    return build("androidpublisher", "v3", credentials=creds)


def in_edit(service, work):
    """Runs work(edits, edit_id) in a Play edit, then commits it."""
    edits = service.edits()
    edit_id = edits.insert(packageName=PACKAGE_NAME, body={}).execute()["id"]
    result = work(edits, edit_id)
    edits.commit(packageName=PACKAGE_NAME, editId=edit_id).execute()
    return result


def release(service, aab, track, notes, fraction):
    from googleapiclient.http import MediaFileUpload

    def work(edits, edit_id):
        media = MediaFileUpload(aab, mimetype="application/octet-stream", resumable=True)
        code = edits.bundles().upload(packageName=PACKAGE_NAME, editId=edit_id, media_body=media).execute()["versionCode"]
        body = {"track": track, "releases": [new_release(code, notes, fraction)]}
        edits.tracks().update(packageName=PACKAGE_NAME, editId=edit_id, track=track, body=body).execute()
        return code

    code = in_edit(service, work)
    reach = f"{fraction:.0%} of users" if fraction else "all users on the track"
    print(f"Version code {code} released to {track}, reaching {reach}.")


def show_status(service, track):
    def work(edits, edit_id):
        return edits.tracks().get(packageName=PACKAGE_NAME, editId=edit_id, track=track).execute()

    for r in in_edit(service, work).get("releases", []):
        reach = f", {r['userFraction']:.0%} of users" if "userFraction" in r else ""
        print(f"{track}: {r.get('versionCodes')} {r.get('status')}{reach}")


def update_rollout(service, track, action, fraction=None):
    def work(edits, edit_id):
        current = edits.tracks().get(packageName=PACKAGE_NAME, editId=edit_id, track=track).execute()
        body = {"track": track, "releases": change_rollout(current.get("releases", []), action, fraction)}
        edits.tracks().update(packageName=PACKAGE_NAME, editId=edit_id, track=track, body=body).execute()

    try:
        in_edit(service, work)
    except ValueError as e:
        sys.exit(str(e))
    print(f"{action} applied to {track}.")


def set_testers(service, track, groups):
    def work(edits, edit_id):
        edits.testers().update(
            packageName=PACKAGE_NAME, editId=edit_id, track=track, body={"googleGroups": groups}
        ).execute()

    in_edit(service, work)
    print(f"Testers on {track} set to: {', '.join(groups)}")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--creds", required=True, help="Path to the service account JSON key")
    parser.add_argument("--track", default=DEFAULT_TRACK, help="internal, alpha, beta or production")
    sub = parser.add_subparsers(dest="command", required=True)

    rel = sub.add_parser("release")
    rel.add_argument("--aab", required=True)
    rel.add_argument("--notes", default="Bug fixes and improvements.")
    rel.add_argument("--fraction", type=parse_fraction, help="Staged rollout, for example 0.1 for 10%%")

    sub.add_parser("status")
    roll = sub.add_parser("rollout")
    roll.add_argument("--fraction", type=parse_fraction, required=True)
    sub.add_parser("halt")
    sub.add_parser("resume")
    sub.add_parser("complete")
    testers = sub.add_parser("testers")
    testers.add_argument("--groups", required=True, nargs="+", help="Google Group email addresses")

    args = parser.parse_args()
    service = get_service(args.creds)
    if args.command == "release":
        release(service, args.aab, args.track, args.notes, args.fraction)
    elif args.command == "status":
        show_status(service, args.track)
    elif args.command == "testers":
        set_testers(service, args.track, args.groups)
    else:
        update_rollout(service, args.track, args.command, getattr(args, "fraction", None))


if __name__ == "__main__":
    main()
