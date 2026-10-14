# nas-mail-client

External application for the NAS launcher, shown as **Email**: it mails the events of the NAS and keeps the sent mails. It does not handle incoming mail.

Until there is an account able to send (server, sender and at least one recipient) the app shows only the first configuration screen. After that, the sidebar holds the mail folders and, at the bottom, **Settings**, a window with two tabs: **Account** (the SMTP server) and **Notifications** (the automatic mails, each with its own switch).

- **SMTP server**: outgoing server, port, security (SSL/TLS, STARTTLS or none), credentials, sender and recipients, with a test mail. The password is encrypted with AES-GCM in the database; the key sits next to the database in a `.key` file with `0600` permissions.
- **Refused sign-ins**: a mail for every sign-in refused for wrong credentials, even when the notifications of the launcher for these sign-ins are off.
- **UI updates**: separate switches for update available, installed and failed.
- **App updates**: a mail when the automatic check of the Application manager finds new versions of the installed apps (event `appUpdatesAvailable`); a check started by hand answers on screen only.
- **System updates**: a mail when apt offers updates, from the chosen minimum level: Security (only `<release>-security`), Stable (also the release and its `-updates`) or All (also backports and other origins).
- **Sent** and **Trash**: the sent mails, including those that did not go out with the reason, can be read, moved to the trash, restored and deleted permanently.

By default the app is not installed. The manifest asks for the Debian package `msmtp` (`permissions.packages`), which the launcher installs before the app and removes when the app is uninstalled, if no other app uses it and it was not already there; and for the `smtp` network (`permissions.network`), which opens ports 25, 465 and 587 to the app in the base firewall. Events come from the launcher on `POST /internal/events` (capability `mail-events`), which the proxy does not serve to the browser; while the app is installed the launcher checks the updates every six hours. With an account configured and at least one notification on, the app tells the launcher (`run.background`, `PUT /api/apps/self/background` on the socket of the launcher, at start and at every save): it stays running with its window closed and starts with the NAS, at low priority. Without, it stops with its window and the launcher starts it again when it has an event to send. The mails are written in the language of the interface at the time the settings were saved.

The app also sends the **account codes** the gateway generates: the verification of the recovery email (Settings of the launcher → Account) and the password reset from the sign-in page. They come as an `accountCode` event on `/internal/events`, go only to the given address in the language of the interface, and need only server and sender (`sender` in `api/status`). They do not stay in Sent: whoever opens the app must not be able to read a code meant for the mailbox. With an account configured, `?section=smtp` opens the settings on the Account tab and `?section=access|ui|apps|system` on the Notifications tab, as the link in the Account section of the launcher does; without, the page stays on the first configuration.

## Local start

Requires Node.js 24, npm and Rust/Cargo. msmtp is needed only to really send.

```bash
pnpm install
pnpm run build
pnpm run backend:dev
```

In a second terminal:

```bash
pnpm run dev
```

Vite serves the UI on `http://127.0.0.1:1440/` and forwards `api/...` to the backend over a Unix socket. `NAS_APP_SOCKET` changes the path of the socket. The SQLite database uses `NAS_DB_PATH` when set, otherwise `$STATE_DIRECTORY/mail-client.sqlite3`; in development it is created in `backend/data/`.

API: `GET api/status` (msmtp present, configuration complete, `sender` for server and sender only), `GET`/`PUT api/settings`, `POST api/test`, `GET api/mails?folder=sent|trash`, `POST api/mails/{id}/trash` and `/restore`, `DELETE api/mails/{id}` (from the trash only), `DELETE api/trash`.

For structure, tests and checks see [DEVELOPMENT.md](DEVELOPMENT.md). The contract with the launcher is in [APPS.md](APPS.md); packaging and release are in [NAS.md](NAS.md).
