#!/usr/bin/env bash
# Real downloads of the Windows installer (Plainly-x64-setup.exe), counted from 20 November
# 2026 (the launch posts on X and Facebook) and kept across rebuilds. Update checks
# (latest.json) are not counted.
#
# Why this is needed: every push rebuilds the installer and recreates the `latest-build`
# release, which resets GitHub's counter. The installer workflow carries the count over in a
# hidden note in the release's description: <!-- downloads: total=T base=B -->, meaning
# T downloads were counted before this file existed, and this file's own counter had already
# reached B when counting began. The real total is T + (this file's count - B).
#
# Usage: scripts/downloads.sh   (needs `gh`, logged in)
set -euo pipefail
gh release view latest-build --repo omikhan4901/text_explainer --json body,assets --jq '
  ((.body | capture("<!-- downloads: total=(?<t>[0-9]+) base=(?<b>[0-9]+) -->")) // {t: "0", b: "0"}) as $m
  | ([.assets[] | select(.name == "Plainly-x64-setup.exe") | .downloadCount][0] // 0) as $c
  | "\(($m.t | tonumber) + ([0, $c - ($m.b | tonumber)] | max)) installer downloads since 20 November 2026"'
