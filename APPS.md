# Contract of nas-mail-client with the NAS launcher

The contract every application shares with the launcher (package, manifest, permissions, frame
protocol, launcher socket and providers) is `docs/development/external_apps/APPS.md` in the
nas-launcher repository, and the bridge is the one of `@valeriogc/nas-base-app`. This file keeps
only what is specific to nas-mail-client, which builds and releases on its own.

## Identity

- id `nas-mail-client`, category `system`
- permissions `network` ["smtp"], `packages` ["msmtp"]
- provides `mail-events` at `/internal/events`

## Specific to this application

nas-mail-client declares the `mail-events` capability at `/internal/events`, the only capability endpoint allowed under `/internal/`, because only the launcher calls it, over the application socket. Each event is `{ key, title, message, params, at }`: `signInRefused`, `launcherUpdateAvailable`, `launcherUpdated`, `launcherRolledBack`, `launcherUpdateFailed`, `launcherUpdateInterrupted`, `appUpdatesAvailable` (with `count` and `apps`, the names, posted only by the automatic check) and `systemUpdatesAvailable` (with `packages`, one `name current -> version (level)` per line). The application answers `202` at once and decides from its own settings whether to send a mail. `accountCode` (`to`, `purpose` `verify` or `reset`, `code`) is the exception: the code of the account goes to `to` at once, the answer is its outcome, and it is not kept with the sent mail. Its manifest also asks for `permissions.network: ["smtp"]` and `permissions.packages: ["msmtp"]`, which the launcher grants as described in its own `APPS.md`.

The commands that check the repository are in [DEVELOPMENT.md](DEVELOPMENT.md).
