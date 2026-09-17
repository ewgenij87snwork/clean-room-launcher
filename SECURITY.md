# Security policy

## Supported versions

| Version | Status |
| --- | --- |
| `0.3.0` | Qualified |
| `0.2.1` | Superseded by `0.3.0` |
| `0.2.0` | Superseded by `0.2.1` |
| `0.1.0-alpha.4.2` | Prior published prerelease |

The v0.3.0 qualification is limited to macOS on Apple Silicon. Its distributed
archive is unsigned and unnotarized at the Apple platform-signing layer.

## Reporting a vulnerability

Start from the repository Security page:
https://github.com/y-sor/clean-room-launcher/security

Use **Security → Report a vulnerability** when that option is available. If the
private reporting option is unavailable, do not put exploit details, credentials,
private paths, prompts, transcripts or user context in a public issue. Open a
minimal public issue asking the maintainer to establish a private reporting
channel.

For a useful report, include the affected CLROOM version, platform, a minimal
reproduction, expected versus observed behavior, and the security impact. Avoid
including unrelated machine or provider state.

The maintainer will acknowledge and triage reports as capacity allows. No fixed
response or remediation SLA is promised.

Please allow reasonable time for investigation and remediation before public
disclosure. Do not publish exploit details or sensitive reproductions while a
report is being triaged.

## Bounty status

This project does not offer a bounty and has no vulnerability rewards program.
Do not incur research cost on the assumption that payment is available.

## Triage data handling

Clean Room Launcher will never request credentials, provider tokens,
transcripts, prompts, private control-plane data or a copy of an unrestricted
home directory for triage.
