# LESSONS.md — QloudiCable

Mistakes already made, and the rule that came out of each. Read before repeating a class of action.

## Public repo: check the diff for private references before committing

An unreleased reference reached a public doc and was pushed. It was fixed in a later commit, but the earlier history still holds internal code. **Rule:** grep staged changes for unreleased or internal names before every commit, and treat anything found as a stop, not a note.

## A classifier can block an outward-facing upload

Creating a separate repo and pushing a local snapshot was refused as a public data upload. **Rule:** do not retry blocked uploads or route around them; keep the material local and ask the owner.

## Heredocs over SSH can run on the wrong machine

An unterminated heredoc sent later commands to the Mac instead of ms7978. **Rule:** check `hostname` and the working directory at the start of each remote command block, and close every heredoc.

## Preview probes can fail silently

Probe scripts in iframes returned no output, so tile taps and sheet open/close were not verified for a while. **Rule:** if a probe prints nothing, that is a failed probe, not a pass. Use a page-level script that logs to the console, and say when an interaction was not verified.

## Selector mismatch looked like a logic bug

A tile lookup returned null because `el()` wrote the `data` prop as a plain attribute. **Rule:** when a selector finds nothing, check the rendered DOM attribute before changing the logic under test.

## "Ahead N" is relative to the tracking branch

`git status` showing "ahead" does not prove what is published. **Rule:** compare with `git rev-list --left-right --count origin/main...HEAD` before telling the owner what is or is not pushed.

## Two checkouts drift

The dashboard lives on ms7978 while this checkout is six behind and one ahead of origin. **Rule:** name the checkout and commit when reporting work; never assume the other one has it.
