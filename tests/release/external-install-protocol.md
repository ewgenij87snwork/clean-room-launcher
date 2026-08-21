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

## Replacement disposition — 2026-08-21

- Result: `BLOCKED_NOT_QUALIFIED`.
- The owner-authorized README correction resolved the prior read-only
  destination blocker. Copy, checksum verification, extraction and staged
  binary identity proof all passed with the original candidate bytes.
- The guided local action returned success but created neither declared local
  state output. The public skill-inspection example rendered no skill decision,
  and the public doctor path reported a missing installation schema that is not
  present in the five-file candidate archive.
- This is a product prerequisite blocker: skill review, deferred-canary
  invocation, saved relaunch, rollback and uninstall cannot be completed.
  No workaround, provider process/request, login, provider-state inspection,
  code fix, documentation change during the run or artifact rebuild occurred.
- The fresh replacement clone was stopped and preserved alongside the earlier
  clone. Temporary media and diagnostic captures were detached/deleted. No
  guest folder permission was granted; guest staging removal was requested but
  its final absence was not asserted after the permission prompt.
