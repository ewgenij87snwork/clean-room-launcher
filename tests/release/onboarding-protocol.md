# Unassisted onboarding protocol

This protocol starts when the participant receives the checksum-bound artifact/link receipt and ends only at a verified clean Codex start. It is an unassisted path: published docs and the one-command path are permitted; coaching, private support, and hidden setup are not.

Record active time, user waiting, help, and errors as separate fields. Do not collapse them into a setup-time claim.

Success requires the participant to be able to state what changed, what did not, how to exit, and how to rollback. These are required comprehension checks; this protocol does not claim they were observed until an authorized external observation exists.

The deterministic internal fixture exercises only the state machine and refusal vocabulary. It is not a human observation. Under the current authority external observation is `NOT_RUN` with reason `OWNER_GATE_REQUIRED_EXTERNAL_USER`; setup-time copy is `UNAVAILABLE`.

Cleanup is required: exit the provider, remove the installed artifact or follow the documented rollback, and verify the clean state. Missing cleanup refuses readiness.
