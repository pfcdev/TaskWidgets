using System.Diagnostics;
using System.Reflection;
using System.Security.Cryptography;
using System.Text.Json.Nodes;

namespace TaskbarStatsProduct;

internal static class GitHubUpdater
{
    private const string Owner = "pfcdev";
    private const string Repo = "TaskWidgets";
    private const string AssetName = "TaskbarStats.exe";
    private const string ShaAssetName = "TaskbarStats.exe.sha256";
    private static readonly string AppDirectory = AppPaths.AppDirectory;
    private static readonly string UpdatesDirectory = Path.Combine(AppDirectory, "Updates");
    private static readonly string LogsDirectory = Path.Combine(AppDirectory, "Logs");
    private static readonly string LogPath = Path.Combine(LogsDirectory, "loader.log");

    public static async Task CheckAndInstallIfAvailableAsync(CancellationToken cancellationToken)
    {
        try
        {
            var release = await GetLatestReleaseAsync(cancellationToken);
            if (release is null || !release.IsNewerThan(CurrentVersion()))
            {
                Log("No GitHub update available");
                return;
            }

            Log($"GitHub update available: {release.TagName}");
            var downloaded = await DownloadReleaseAsync(release, cancellationToken);
            StartUpdateScriptAndExit(downloaded);
        }
        catch (OperationCanceledException)
        {
            throw;
        }
        catch (Exception ex)
        {
            Log($"Update check failed: {ex.Message}");
        }
    }

    public static async Task CheckOnlyAsync(CancellationToken cancellationToken)
    {
        try
        {
            var release = await GetLatestReleaseAsync(cancellationToken);
            if (release is null)
            {
                Log("No GitHub release found");
                return;
            }

            var current = CurrentVersion();
            Log(release.IsNewerThan(current)
                ? $"Update available: current={current}, latest={release.TagName}"
                : $"Already current: current={current}, latest={release.TagName}");
        }
        catch (Exception ex)
        {
            Log($"Update check failed: {ex.Message}");
        }
    }

    private static async Task<ReleaseInfo?> GetLatestReleaseAsync(CancellationToken cancellationToken)
    {
        using var client = CreateHttpClient();
        var url = $"https://api.github.com/repos/{Owner}/{Repo}/releases/latest";
        using var response = await client.GetAsync(url, cancellationToken);
        if (!response.IsSuccessStatusCode)
        {
            throw new InvalidOperationException(
                $"GitHub latest release request failed: {(int)response.StatusCode} {response.ReasonPhrase}");
        }

        var json = JsonNode.Parse(await response.Content.ReadAsStringAsync(cancellationToken)) ??
                   throw new InvalidOperationException("GitHub latest release response was empty");
        var tag = json["tag_name"]?.GetValue<string>();
        if (string.IsNullOrWhiteSpace(tag))
        {
            return null;
        }

        string? exeUrl = null;
        string? shaUrl = null;
        foreach (var asset in json["assets"]?.AsArray() ?? [])
        {
            var name = asset?["name"]?.GetValue<string>();
            var downloadUrl = asset?["browser_download_url"]?.GetValue<string>();
            if (string.IsNullOrWhiteSpace(name) || string.IsNullOrWhiteSpace(downloadUrl))
            {
                continue;
            }

            if (string.Equals(name, AssetName, StringComparison.OrdinalIgnoreCase))
            {
                exeUrl = downloadUrl;
            }
            else if (string.Equals(name, ShaAssetName, StringComparison.OrdinalIgnoreCase))
            {
                shaUrl = downloadUrl;
            }
        }

        return string.IsNullOrWhiteSpace(exeUrl)
            ? null
            : new ReleaseInfo(tag, exeUrl, shaUrl);
    }

    private static async Task<string> DownloadReleaseAsync(
        ReleaseInfo release,
        CancellationToken cancellationToken)
    {
        Directory.CreateDirectory(UpdatesDirectory);
        var directory = Path.Combine(UpdatesDirectory, release.TagName);
        Directory.CreateDirectory(directory);
        var exePath = Path.Combine(directory, AssetName);

        using var client = CreateHttpClient();
        var bytes = await client.GetByteArrayAsync(release.DownloadUrl, cancellationToken);
        if (bytes.Length < 1024 * 1024)
        {
            throw new InvalidOperationException("Downloaded update is unexpectedly small");
        }

        await File.WriteAllBytesAsync(exePath, bytes, cancellationToken);

        if (!string.IsNullOrWhiteSpace(release.Sha256Url))
        {
            var shaText = await client.GetStringAsync(release.Sha256Url, cancellationToken);
            var expected = shaText.Split([' ', '\t', '\r', '\n'], StringSplitOptions.RemoveEmptyEntries)
                .FirstOrDefault();
            if (!string.IsNullOrWhiteSpace(expected))
            {
                var actual = Convert.ToHexString(SHA256.HashData(bytes)).ToLowerInvariant();
                if (!string.Equals(expected.Trim().ToLowerInvariant(), actual, StringComparison.OrdinalIgnoreCase))
                {
                    throw new InvalidOperationException("Downloaded update SHA256 verification failed");
                }
            }
        }

        Log($"Downloaded update {release.TagName} to {exePath}");
        return exePath;
    }

    private static void StartUpdateScriptAndExit(string downloadedExe)
    {
        var currentExe = Environment.ProcessPath ??
                         Process.GetCurrentProcess().MainModule?.FileName ??
                         throw new InvalidOperationException("Current executable path could not be resolved");
        var scriptPath = Path.Combine(Path.GetDirectoryName(downloadedExe)!, "apply-update.cmd");
        var currentPid = Environment.ProcessId;
        var script = $"""
@echo off
setlocal
set "SRC={downloadedExe}"
set "DST={currentExe}"
set "PID={currentPid}"
:wait
tasklist /FI "PID eq %PID%" | find "%PID%" >nul
if not errorlevel 1 (
  timeout /t 1 /nobreak >nul
  goto wait
)
copy /Y "%SRC%" "%DST%" >nul
start "" "%DST%"
del "%~f0"
""";
        File.WriteAllText(scriptPath, script);

        Process.Start(new ProcessStartInfo
        {
            FileName = "cmd.exe",
            UseShellExecute = false,
            CreateNoWindow = true,
            ArgumentList = { "/c", scriptPath }
        });

        Log("Update apply script started; exiting current process");
        Environment.Exit(0);
    }

    private static HttpClient CreateHttpClient()
    {
        var client = new HttpClient();
        client.DefaultRequestHeaders.UserAgent.ParseAdd("TaskbarStats/0.1");
        client.DefaultRequestHeaders.Accept.ParseAdd("application/vnd.github+json");
        return client;
    }

    private static Version CurrentVersion() =>
        Assembly.GetExecutingAssembly().GetName().Version ?? new Version(0, 0, 0);

    private static void Log(string message)
    {
        try
        {
            Directory.CreateDirectory(LogsDirectory);
            File.AppendAllText(
                LogPath,
                $"{DateTimeOffset.Now:O} [updater] {message}{Environment.NewLine}");
        }
        catch
        {
            // Logging must never break update checks.
        }
    }

    private sealed record ReleaseInfo(string TagName, string DownloadUrl, string? Sha256Url)
    {
        public bool IsNewerThan(Version current)
        {
            var normalized = TagName.TrimStart('v', 'V');
            return Version.TryParse(normalized, out var latest) && latest > current;
        }
    }
}
