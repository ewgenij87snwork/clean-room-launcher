# Upgrade and remove the local preview

The local preview is an extracted archive. It does not install a daemon,
service, account, or system-wide configuration.

To try a newer archive, verify its SHA-256, extract it into a new directory,
and invoke that directory's `bin/clroom`. Keep the previous directory until
you have finished the new session.

To roll back, invoke the previous archive directory again:

```sh
./clean-room-launcher-previous/bin/clroom status
```

To remove a preview, delete only its extracted archive directory. This does
not modify your Codex authentication, existing configuration, project files,
or provider installation.
