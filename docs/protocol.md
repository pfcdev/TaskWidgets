# Protocol

The current prototype uses a local JSON handoff file instead of a named pipe:

```text
%LOCALAPPDATA%\TaskbarStats\codex-status.json
```

The file is written by `CodexStatusAgent` and read by the Windhawk taskbar mod.
Messages are versioned so the taskbar module can ignore unsupported payloads
without crashing Explorer.

```json
{
  "version": 1,
  "updatedAtUnix": 1783785846,
  "status": "ok",
  "error": null,
  "planType": "plus",
  "primaryUsedPercent": 36,
  "primaryWindowMins": 300,
  "primaryResetsAtUnix": 1783802904,
  "secondaryUsedPercent": 6,
  "secondaryWindowMins": 10080,
  "secondaryResetsAtUnix": 1784389704,
  "creditsBalance": "0",
  "creditsUnlimited": false,
  "tokens30d": 884413232,
  "threadCount30d": 36
}
```

`primary*` represents the short Codex rate-limit window. `secondary*`
represents the longer window. The taskbar slider currently renders `LIMIT`,
`WEEK`, `30D`, and `PLAN` views from these fields.
