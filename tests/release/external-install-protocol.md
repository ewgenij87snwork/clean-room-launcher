# External clean-install protocol

This protocol is controller-only evidence for canonical P08 Task 8. It is not
provided to the tester and must not be used to coach or repair the public
README path.

## Fixed subject

- Candidate: `clean-room-launcher-v0.1.0-aarch64-apple-darwin.tar.gz`
- Candidate SHA-256:
  `a8ed1ab765a64cec6cca2e6acb7820f137753b09513194f1cdbe4f250dfd0986`
- Tester inputs: the candidate, `SHA256SUMS`, and `README.md`, and no repository
  checkout or private instructions.
- Environment: one disposable full macOS clone, standard account `user-clean`,
  Guest Tools absent, host sharing and clipboard disabled.

## Fail-closed sequence

1. Prove the source VM is stopped at the named clean baseline snapshot.
2. Create one full clone without linked-clone mode. Prove distinct VM identity,
   bundle and disk paths, and no external parent-disk reference.
3. Attach a temporary read-only image containing exactly the three fixed inputs.
4. In the guest, use `user-clean`; copy no host repository or private state.
5. Follow `README.md` without controller coaching. Verify `SHA256SUMS` before
   extraction and prove all launches resolve under the staged artifact root.
6. Exercise only the Task 8 install/lifecycle path: install, clean provider
   start, skill review, deferred-canary invocation, saved standard relaunch,
   rollback, uninstall, and final cleanup. At most one provider request is
   permitted; login, provider-state inspection, credentials and Keychain access
   are forbidden.
7. A missing prerequisite, login/provider requirement, incomplete README step,
   workaround, or uncompleted lifecycle step yields `BLOCKED_NOT_QUALIFIED`.
   Do not repair code, alter documentation, rebuild the candidate, or retry a
   provider request under this authority.
8. Shut down the disposable clone after the result. Preserve it for owner
   disposition; do not delete it.

## Receipt boundary

Persist only fixed artifact hashes, booleans, sanitized status codes, counts and
timings. Do not retain raw provider output, credentials, guest private data,
browser/login state, or host/guest private paths. A `PASS` requires every step;
partial success never upgrades `BLOCKED_NOT_QUALIFIED`.

## Observed disposition — 2026-08-21

- Result: `BLOCKED_NOT_QUALIFIED`.
- The exact candidate checksum passed before extraction.
- The immediately following README extraction command was run from the
  read-only staged medium and could not create its destination. The public
  README does not direct the tester to a writable working directory.
- No workaround, provider process, provider request, login, provider-state
  inspection, code fix, documentation fix or artifact rebuild was attempted.
- The disposable clone was shut down and preserved. Temporary media was
  detached after its container digest was rechecked unchanged.
