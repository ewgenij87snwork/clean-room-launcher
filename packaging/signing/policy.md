# Signing and notarization policy

TaskSeal records three artifact states. A state is accepted only from the
platform verifier operating on the exact artifact bytes; the presence of a
signature-shaped file or metadata field is never evidence.

- `unsigned`: the platform verifier reports either no signature or a macOS
  ad-hoc signature with no authenticated identity. The ad-hoc mechanism is
  recorded explicitly and can never satisfy `signed` or `signed+notarized`.
  This state is usable only as an explicitly labelled unsigned preview and
  remains `NOT_QUALIFIED`.
- `signed`: the platform verifier accepts the artifact and the observed signer
  identity exactly matches the expected identity. Ad-hoc signatures, an
  unknown identity, or a verification error are refusals for authenticated
  claims.
- `signed+notarized`: macOS `codesign` verification and exact identity match
  both pass, and `spctl --assess --type execute` reports a notarized Developer
  ID source for the same artifact.

The macOS verifier commands are `codesign --verify --deep --strict` followed
by `codesign -dv` for identity and, for notarization, `spctl --assess --type
execute`. Windows verification uses PowerShell `Get-AuthenticodeSignature` and
requires `Status=Valid` plus an exact signer subject. Credentials and signing
commands are deliberately absent: this policy verifies artifacts but never
creates signatures or notarization submissions.

If a platform tool or approved identity is unavailable, the lane is
`NOT_QUALIFIED`; it is never skipped or promoted from file presence alone.
