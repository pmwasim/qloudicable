#!/usr/bin/env python3
"""Checks the rollout logic in play-console-automation.py, without contacting Google."""

import argparse
import importlib.util
from pathlib import Path

spec = importlib.util.spec_from_file_location(
    "play", Path(__file__).with_name("play-console-automation.py")
)
play = importlib.util.module_from_spec(spec)
spec.loader.exec_module(play)


def raises(error, fn, *args):
    try:
        fn(*args)
    except error:
        return True
    return False


live = {"versionCodes": ["1000"], "status": "completed"}
staged = {"versionCodes": ["1001"], "status": "inProgress", "userFraction": 0.1}
halted = {"versionCodes": ["1001"], "status": "halted", "userFraction": 0.1}

# A staged release is built only when a fraction is given.
assert play.new_release(5, "n")["status"] == "completed"
assert "userFraction" not in play.new_release(5, "n")
assert play.new_release(5, "n", 0.2)["status"] == "inProgress"
assert play.new_release(5, "n", 0.2)["userFraction"] == 0.2

# Each action changes only the staged release.
assert play.change_rollout([live, staged], "rollout", 0.5)[1]["userFraction"] == 0.5
assert play.change_rollout([live, staged], "halt")[1]["status"] == "halted"
assert play.change_rollout([live, staged], "halt")[0] == live
assert play.change_rollout([live, halted], "resume")[1]["status"] == "inProgress"
done = play.change_rollout([staged], "complete")[0]
assert done["status"] == "completed" and "userFraction" not in done
assert play.change_rollout([halted], "complete")[0]["status"] == "completed"

# The caller's data is left alone.
assert staged["status"] == "inProgress"

# Nothing to act on is an error, not a silent success.
assert raises(ValueError, play.change_rollout, [live], "halt")
assert raises(ValueError, play.change_rollout, [live, staged], "resume")
assert raises(ValueError, play.change_rollout, [staged, dict(staged)], "halt")
assert raises(ValueError, play.change_rollout, [staged], "rollout")

assert play.parse_fraction("0.1") == 0.1
for bad in ("0", "1", "1.5", "-0.1"):
    assert raises(argparse.ArgumentTypeError, play.parse_fraction, bad)

print("Release tooling checks passed.")
