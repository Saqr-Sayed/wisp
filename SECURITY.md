# Security

Wisp stores your activity locally and never sends data anywhere; most
"security issues" here are actually privacy expectations, and we take both
seriously.

## Reporting a vulnerability

Please **do not** open a public issue for security problems. Instead, email
the maintainer directly (visible on the GitHub profile) or use the private
vulnerability reporting form on the repository's **Security** tab — either
way, we respond promptly and will credit you in the release notes.

## Being careful with your data

- The SQLite database is stored unencrypted on disk (plain file at the paths
  in the README). On a shared or untrusted machine, encrypt your disk or move
  the DB location to an encrypted volume.
- The WebView/D-Bus channels are local only; nothing in Wisp makes outbound
  network calls.

## Scope

Only the code in this repository is in scope. The GNOME extension ships with
no network capability; the Windows builds are unsigned — the checksums are in
the release notes, verify them if you download installers elsewhere.