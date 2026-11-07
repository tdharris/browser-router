# Browser Router

Browser Router is a small Windows 10/11 URL dispatcher. It registers as a
browser for the current user, evaluates local day/time and hostname rules, then
starts the selected browser without showing a console window.

[Download the latest Windows release](https://github.com/tdharris/browser-router/releases/latest)

The release is one statically linked Rust executable with no runtime, service,
background process, administrator access, or UAC prompt.

## Architecture

Each invocation performs a short, synchronous path:

1. Windows invokes `browser-router.exe "<URL>"` through the registered
   `BrowserRouterURL` protocol.
2. The router reads `%APPDATA%\browser-router\config.toml`.
3. Rules are evaluated in file order against local time and the URL hostname.
4. The selected browser is created detached with `DETACHED_PROCESS` and
   `CREATE_NO_WINDOW`; the router exits immediately.

Malformed configuration falls back to Microsoft Edge and is recorded in
`%APPDATA%\browser-router\browser-router.log`. If a configured browser cannot
be started, the router asks Windows to open the URL and uses the Edge protocol
as a final recursion-safe fallback.

## Install

### Installer

Download `browser-router-<version>-setup.exe` from the
[Releases page](https://github.com/tdharris/browser-router/releases), run it,
then enable **Browser Router** for both HTTP and HTTPS when Windows Default Apps
opens. Installation and registry changes are per-user.

### Portable

Place `browser-router.exe` in a permanent directory and run:

```powershell
.\browser-router.exe --register
start ms-settings:defaultapps?app=BrowserRouter
```

Choose Browser Router for both `HTTP` and `HTTPS`. Do not move the executable
after registration; unregister and register again if its location changes.

To remove the registration:

```powershell
.\browser-router.exe --unregister
```

Unregistering does not delete the user's configuration.

## Configuration

The first run creates a documented file at:

```text
%APPDATA%\browser-router\config.toml
```

`config.toml` in this repository is a fuller example. The default browser can
be a table with fixed arguments:

```toml
[default_browser]
path = 'C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe'
args = []
```

It may also be abbreviated as `default_browser = "firefox.exe"`.

Add routing rules with `[[rules]]`:

```toml
[[rules]]
name = "Work profile"
browser = 'C:\Program Files\Google\Chrome\Application\chrome.exe'
args = ["--profile-directory=Work"]
days = ["Mon", "Tue", "Wed", "Thu", "Fri"]
time_range = ["08:00", "18:00"]
domains = ["*.internal.net", "*corp.com*"]
```

Rules are first-match-wins. All filters on a rule must match:

- `days`: three-letter or full English names, case-insensitive. Empty or
  omitted means every day.
- `time_range`: local 24-hour time, start-inclusive and end-exclusive. A range
  such as `["22:00", "02:00"]` crosses midnight. Equal times mean all day.
- `domains`: case-insensitive hostname globs where `*` matches any sequence and
  `?` matches one character. Empty or omitted means every hostname.
- Regex patterns use the explicit `regex:` prefix, for example
  `'regex:^jira[0-9]+\.example\.com$'`. `/pattern/` is also accepted.

Fixed browser arguments are passed directly to the executable and the URL is
appended as one argument. No command shell is involved.

## Build

The project uses Rust 2024 and pins Rust 1.99.1 through asdf:

```bash
asdf install
cargo test
```

The production target is MSVC:

```powershell
rustup target add x86_64-pc-windows-msvc
cargo build --release --locked --target x86_64-pc-windows-msvc
```

Building the MSVC target normally requires the Microsoft linker and Windows
SDK, so use a Windows developer shell or the included GitHub Actions workflow.
The workflow tests, builds the standalone EXE, creates a ZIP, builds the Inno
Setup installer, uploads all artifacts, and publishes them for `v*` tags.

To build the installer locally, install Inno Setup 6, build the release binary,
and compile `installer.iss`.

## Registry scope

`--register` writes only beneath `HKEY_CURRENT_USER`:

- `Software\Clients\StartMenuInternet\BrowserRouter`
- `Software\RegisteredApplications` (`BrowserRouter` value)
- `Software\Classes\BrowserRouterURL`

Windows retains control of the actual user-choice association. The router does
not modify the protected `UserChoice` key or bypass the Default Apps UI.

## Troubleshooting

### Windows changed the default browser

Feature updates, browser installations, and browser self-checks can reset URL
associations. Open:

```powershell
start ms-settings:defaultapps?app=BrowserRouter
```

Reassign Browser Router to both HTTP and HTTPS. If it is missing, run
`--register` again from the executable's permanent location.

### A link opens Edge

Check `%APPDATA%\browser-router\browser-router.log`. Common causes are invalid
TOML, a misspelled browser path, or a browser that was moved by an update.
Compare the active file with the repository's `config.toml`.

### A rule does not match

Rules match only the hostname, not the path, query string, or URL scheme.
Remember that `*.example.com` matches `team.example.com` but not the bare
`example.com`; list both patterns when both should match.

### Registration is stale

Run `--unregister`, confirm the executable is in its final location, then run
`--register` and revisit Default Apps.
