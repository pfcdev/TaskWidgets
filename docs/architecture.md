# Architecture

## Current Prototype

`windhawk/taskbar-stats.wh.cpp` is a Windhawk mod for `explorer.exe`.

The mod:

- waits for Windows 11 taskbar XAML host windows to exist;
- injects a XAML Diagnostics TAP into the current Explorer process;
- watches XAML visual tree additions;
- finds `SystemTray.SystemTrayFrame`;
- inserts a `StackPanel` named `TaskbarStatsRoot` into the same taskbar
  parent grid;
- reads `%LOCALAPPDATA%\TaskbarStats\codex-status.json`;
- rotates compact Codex status slides on a lightweight XAML timer;
- removes the inserted UI and restores shifted grid columns on unload.

The status collection work runs outside Explorer in
`agent/CodexStatusAgent`. The Windhawk side only reads a small local JSON file
and updates existing text controls.

## CodexStatusAgent

The agent is a .NET console app. It:

- starts `codex app-server` on a temporary loopback WebSocket port;
- sends `initialize`, `initialized`, and `account/rateLimits/read`;
- extracts Codex rate-limit windows from the returned `RateLimitSnapshot`;
- reads local 30-day token totals from `%USERPROFILE%\.codex\state_5.sqlite`
  via `sqlite3.exe` when it is available;
- writes `%LOCALAPPDATA%\TaskbarStats\codex-status.json` atomically.

## Private Windows Assumptions

Windows 11 taskbar internals are private and build-dependent. This prototype
assumes that a taskbar XAML element with the type name
`SystemTray.SystemTrayFrame` appears in the visual tree, and that its parent is
a XAML `Grid` that can accept an auto-width column immediately before the tray
column.

If these assumptions are wrong for a Windows build, the mod logs a diagnostic
message and leaves the taskbar unchanged.

## Product Loader

`product/TaskbarStats` is the Windhawk-free product loader. It publishes a
single-file `.NET 8` `TaskbarStats.exe` that embeds the native hook DLL,
extracts it to `%LOCALAPPDATA%\TaskbarStats\Runtime\...`, injects it into the
current-session `explorer.exe`, runs the Codex status collector loop, and
watches Explorer for restarts.

`product/TaskbarStatsHook` is the native Explorer-side hook DLL. It is ported
from the Windhawk prototype, writes diagnostics to
`%LOCALAPPDATA%\TaskbarStats\Logs\hook.log`, and uses a named shutdown event so
the loader can detach it without Windhawk.

## Updates

The loader checks `https://api.github.com/repos/pfcdev/TaskWidgets/releases/latest`
for GitHub Releases. A release is considered installable when it has a newer
semantic tag such as `v0.1.1` and includes `TaskbarStats.exe`. If the optional
`TaskbarStats.exe.sha256` asset exists, the downloaded executable is verified
before installation. The updater writes a small command script under
`%LOCALAPPDATA%\TaskbarStats\Updates\...`, waits for the current process to
exit, replaces the executable in place, and restarts it.
