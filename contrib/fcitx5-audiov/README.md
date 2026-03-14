# fcitx5 Audiov Addon

This addon exposes a small D-Bus method inside the main `org.fcitx.Fcitx5`
service:

- path: `/org/freedesktop/Fcitx5/Audiov`
- interface: `org.fcitx.Fcitx5.Audiov1`
- method: `CommitText(s text) -> b`

When `audiov` calls this method, the addon commits the text to the currently
focused `fcitx5` input context. This is intended for Ghostty and other clients
that already integrate with the `fcitx5` IME stack.

## User Install

```bash
./contrib/fcitx5-audiov/install-user.sh
```

This installs the addon into `~/.local`, then rewrites the installed
`audiovfcitx5.conf` so `fcitx5` loads the shared library from an absolute path.
That avoids the system-wide `/usr/lib/fcitx5` requirement.

## Restart fcitx5

```bash
./contrib/fcitx5-audiov/restart-fcitx5.sh
```

## Verify addon object

```bash
busctl --user introspect org.fcitx.Fcitx5 /org/freedesktop/Fcitx5/Audiov
```

You should see `org.fcitx.Fcitx5.Audiov1` with a `CommitText` method.

## Audiov Config

```toml
[paste]
mode = "fcitx5"
command = []
fcitx5_service = "org.fcitx.Fcitx5"
fcitx5_path = "/org/freedesktop/Fcitx5/Audiov"
fcitx5_interface = "org.fcitx.Fcitx5.Audiov1"
```

## Manual smoke test

Focus a text field that is backed by `fcitx5`, then run:

```bash
busctl --user call org.fcitx.Fcitx5 \
  /org/freedesktop/Fcitx5/Audiov \
  org.fcitx.Fcitx5.Audiov1 \
  CommitText s "audiov-fcitx5-smoke"
```
