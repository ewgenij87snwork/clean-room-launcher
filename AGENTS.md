# TaskSeal repository execution law

- Before any write, read `.taskseal-dev/execution-authority.json`. If it is
  absent or invalid, do not mutate; report `WAITING_FOR_OWNER`.
- Read and follow the absolute execution protocol, v2 index and assigned plan
  named by that authority. Conversation history is not authority.
- Verify physical `pwd -P`, `git rev-parse --show-toplevel`, repository root,
  non-main branch, HEAD, worktree,
  checkpoint and allowed task range before writing and after every continuation,
  compaction, reattach, provider change or handoff.
- Mutate only the assigned plan's exclusive write-set in the exact approved
  worktree. Preserve unrelated dirty/untracked/stashed work.
- Use `scripts/gates/pxx/verify.sh` as the sole plan completion gate. Two
  unchanged failures stop retries and require diagnosis/change of input.
- Update the authority-named dashboard, status and append-only worklog at every
  start, accepted task, material change, blocker, handoff and stop.
- Every owner update begins with the protocol's two percentages and includes
  version, human plan/task, user result, real check, worktree/branch/HEAD, time,
  safe parallel work and exact next action.
- Never mutate `main`, publish, spend, contact external parties, integrate into
  Praxis or delete a worktree without a separate exact owner authorization.
