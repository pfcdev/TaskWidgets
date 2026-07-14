using System.Diagnostics;
using System.Drawing;
using System.IO.Compression;
using System.Reflection;
using Microsoft.Win32;
using System.Windows.Forms;

namespace TaskbarStatsInstaller;

internal static class Program
{
    private const string AppName = "TaskbarStats";
    private static readonly string Version =
        Assembly.GetExecutingAssembly().GetName().Version?.ToString(3) ?? "0.1.0";
    private const string PackageResource = "TaskbarStatsPackage.zip";
    private const string StartupValueName = "TaskbarStats";
    private const string UninstallKeyPath =
        @"Software\Microsoft\Windows\CurrentVersion\Uninstall\TaskbarStats";

    private static string InstallDir = DefaultInstallDir();

    private static readonly string ProgramsDir = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.StartMenu),
        "Programs",
        "TaskbarStats");

    [STAThread]
    private static int Main(string[] args)
    {
        bool quiet = HasArg(args, "/quiet") || HasArg(args, "--quiet");
        bool noStart = HasArg(args, "/no-start") || HasArg(args, "--no-start");
        bool uninstall = HasArg(args, "/uninstall") || HasArg(args, "--uninstall");
        InstallDir = ResolveInstallDir(args, uninstall);

        try
        {
            if (uninstall)
            {
                Uninstall(quiet);
                return 0;
            }

            if (!quiet)
            {
                Application.EnableVisualStyles();
                Application.SetCompatibleTextRenderingDefault(false);
                using var form = new InstallForm(InstallDir, !noStart);
                if (form.ShowDialog() != DialogResult.OK)
                {
                    return 0;
                }

                InstallDir = form.InstallDirectory;
                noStart = !form.StartAfterInstall;
            }

            Install(noStart);
            if (!quiet)
            {
                MessageBox.Show(
                    "TaskbarStats kuruldu ve başlatıldı.",
                    "TaskbarStats Setup",
                    MessageBoxButtons.OK,
                    MessageBoxIcon.Information);
            }

            return 0;
        }
        catch (Exception ex)
        {
            if (!quiet)
            {
                MessageBox.Show(
                    ex.Message,
                    "TaskbarStats Setup",
                    MessageBoxButtons.OK,
                    MessageBoxIcon.Error);
            }

            return 1;
        }
    }

    private static bool HasArg(string[] args, string value) =>
        args.Any(arg => string.Equals(arg, value, StringComparison.OrdinalIgnoreCase));

    private static string DefaultInstallDir() => Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
        "Programs",
        "TaskbarStats");

    private static string ResolveInstallDir(string[] args, bool uninstall)
    {
        foreach (string arg in args)
        {
            const string slashPrefix = "/dir=";
            const string dashPrefix = "--install-dir=";
            if (arg.StartsWith(slashPrefix, StringComparison.OrdinalIgnoreCase))
            {
                return Path.GetFullPath(arg[slashPrefix.Length..].Trim('"'));
            }
            if (arg.StartsWith(dashPrefix, StringComparison.OrdinalIgnoreCase))
            {
                return Path.GetFullPath(arg[dashPrefix.Length..].Trim('"'));
            }
        }

        if (uninstall)
        {
            string? self = Environment.ProcessPath;
            string? selfDir = string.IsNullOrWhiteSpace(self) ? null : Path.GetDirectoryName(self);
            if (!string.IsNullOrWhiteSpace(selfDir) &&
                File.Exists(Path.Combine(selfDir, "TaskbarStats.exe")))
            {
                return selfDir;
            }

            using RegistryKey? key = Registry.CurrentUser.OpenSubKey(UninstallKeyPath);
            string? registered = key?.GetValue("InstallLocation") as string;
            if (!string.IsNullOrWhiteSpace(registered))
            {
                return registered;
            }
        }

        return DefaultInstallDir();
    }

    private static void Install(bool noStart)
    {
        StopTaskbarStatsProcesses();

        Directory.CreateDirectory(InstallDir);
        ExtractPackage();
        CopySelfForUninstall();
        CreateShortcuts();
        RegisterStartup();
        RegisterUninstall();

        if (!noStart)
        {
            StartInstalledApp();
        }
    }

    private static void Uninstall(bool quiet)
    {
        StopTaskbarStatsProcesses();
        RemoveStartup();
        RemoveShortcuts();
        RemoveUninstallRegistration();

        string self = Environment.ProcessPath ?? string.Empty;
        bool runningFromInstallDir =
            !string.IsNullOrWhiteSpace(self) &&
            self.StartsWith(InstallDir, StringComparison.OrdinalIgnoreCase);

        if (runningFromInstallDir)
        {
            ScheduleInstallDirRemoval();
        }
        else if (Directory.Exists(InstallDir))
        {
            Directory.Delete(InstallDir, recursive: true);
        }

        if (!quiet)
        {
            MessageBox.Show(
                "TaskbarStats kaldırıldı.",
                "TaskbarStats Setup",
                MessageBoxButtons.OK,
                MessageBoxIcon.Information);
        }
    }

    private static void ExtractPackage()
    {
        using Stream stream = Assembly.GetExecutingAssembly()
                                  .GetManifestResourceStream(PackageResource) ??
                              throw new InvalidOperationException(
                                  "Installer paketi bulunamadı.");
        using var archive = new ZipArchive(stream, ZipArchiveMode.Read);
        foreach (var entry in archive.Entries)
        {
            if (string.IsNullOrEmpty(entry.Name))
            {
                continue;
            }

            string destination = Path.GetFullPath(Path.Combine(InstallDir, entry.FullName));
            string root = Path.GetFullPath(InstallDir) + Path.DirectorySeparatorChar;
            if (!destination.StartsWith(root, StringComparison.OrdinalIgnoreCase))
            {
                throw new InvalidOperationException("Installer paket yolu geçersiz.");
            }

            Directory.CreateDirectory(Path.GetDirectoryName(destination)!);
            entry.ExtractToFile(destination, overwrite: true);
        }
    }

    private static void CopySelfForUninstall()
    {
        string? self = Environment.ProcessPath;
        if (string.IsNullOrWhiteSpace(self) || !File.Exists(self))
        {
            return;
        }

        string target = Path.Combine(InstallDir, "TaskbarStatsSetup.exe");
        if (!string.Equals(self, target, StringComparison.OrdinalIgnoreCase))
        {
            File.Copy(self, target, overwrite: true);
        }
    }

    private static void CreateShortcuts()
    {
        Directory.CreateDirectory(ProgramsDir);
        string app = Path.Combine(InstallDir, "TaskbarStats.exe");
        string settings = Path.Combine(InstallDir, "TaskbarStatsSettings.exe");
        string installer = Path.Combine(InstallDir, "TaskbarStatsSetup.exe");

        CreateShortcut(
            Path.Combine(ProgramsDir, "TaskbarStats.lnk"),
            app,
            "",
            InstallDir,
            "TaskbarStats");
        CreateShortcut(
            Path.Combine(ProgramsDir, "TaskbarStats Settings.lnk"),
            settings,
            "",
            InstallDir,
            "TaskbarStats Settings");
        CreateShortcut(
            Path.Combine(ProgramsDir, "Uninstall TaskbarStats.lnk"),
            installer,
            "/uninstall",
            InstallDir,
            "Uninstall TaskbarStats");
    }

    private static void CreateShortcut(
        string shortcutPath,
        string targetPath,
        string arguments,
        string workingDirectory,
        string description)
    {
        try
        {
            Type? shellType = Type.GetTypeFromProgID("WScript.Shell");
            if (shellType is null)
            {
                return;
            }

            dynamic shell = Activator.CreateInstance(shellType)!;
            dynamic shortcut = shell.CreateShortcut(shortcutPath);
            shortcut.TargetPath = targetPath;
            shortcut.Arguments = arguments;
            shortcut.WorkingDirectory = workingDirectory;
            shortcut.Description = description;
            shortcut.IconLocation = targetPath;
            shortcut.Save();
        }
        catch
        {
            // Shortcuts are convenience only; installation can continue.
        }
    }

    private static void RegisterStartup()
    {
        using RegistryKey key = Registry.CurrentUser.OpenSubKey(
                                    @"Software\Microsoft\Windows\CurrentVersion\Run",
                                    writable: true) ??
                                Registry.CurrentUser.CreateSubKey(
                                    @"Software\Microsoft\Windows\CurrentVersion\Run",
                                    writable: true);
        key.SetValue(StartupValueName, $"\"{Path.Combine(InstallDir, "TaskbarStats.exe")}\"");
    }

    private static void RemoveStartup()
    {
        using RegistryKey? key = Registry.CurrentUser.OpenSubKey(
            @"Software\Microsoft\Windows\CurrentVersion\Run",
            writable: true);
        key?.DeleteValue(StartupValueName, throwOnMissingValue: false);
    }

    private static void RegisterUninstall()
    {
        using RegistryKey key = Registry.CurrentUser.CreateSubKey(UninstallKeyPath, writable: true);
        string installer = Path.Combine(InstallDir, "TaskbarStatsSetup.exe");
        key.SetValue("DisplayName", "TaskbarStats");
        key.SetValue("DisplayVersion", Version);
        key.SetValue("Publisher", "TaskbarStats");
        key.SetValue("InstallLocation", InstallDir);
        key.SetValue("DisplayIcon", Path.Combine(InstallDir, "TaskbarStats.exe"));
        key.SetValue("UninstallString", $"\"{installer}\" /uninstall");
        key.SetValue("QuietUninstallString", $"\"{installer}\" /uninstall /quiet");
        key.SetValue("NoModify", 1, RegistryValueKind.DWord);
        key.SetValue("NoRepair", 1, RegistryValueKind.DWord);
    }

    private static void RemoveUninstallRegistration()
    {
        Registry.CurrentUser.DeleteSubKeyTree(UninstallKeyPath, throwOnMissingSubKey: false);
    }

    private static void RemoveShortcuts()
    {
        if (Directory.Exists(ProgramsDir))
        {
            Directory.Delete(ProgramsDir, recursive: true);
        }
    }

    private static void StopTaskbarStatsProcesses()
    {
        int current = Environment.ProcessId;
        foreach (string name in new[]
                 {
                     "TaskbarStats",
                     "TaskbarStatsMediaHelper",
                     "TaskbarStatsSettings"
                 })
        {
            foreach (Process process in Process.GetProcessesByName(name))
            {
                using (process)
                {
                    if (process.Id == current)
                    {
                        continue;
                    }

                    try
                    {
                        process.Kill(entireProcessTree: true);
                        process.WaitForExit(5000);
                    }
                    catch
                    {
                        // Best effort. File copy will report a real error if it stays locked.
                    }
                }
            }
        }
    }

    private static void StartInstalledApp()
    {
        string app = Path.Combine(InstallDir, "TaskbarStats.exe");
        if (!File.Exists(app))
        {
            return;
        }

        Process.Start(new ProcessStartInfo
        {
            FileName = app,
            WorkingDirectory = InstallDir,
            UseShellExecute = true,
            WindowStyle = ProcessWindowStyle.Hidden
        });
    }

    private static void ScheduleInstallDirRemoval()
    {
        string command =
            $"/c timeout /t 2 /nobreak >nul & rmdir /s /q \"{InstallDir}\"";
        Process.Start(new ProcessStartInfo
        {
            FileName = "cmd.exe",
            Arguments = command,
            CreateNoWindow = true,
            UseShellExecute = false,
            WindowStyle = ProcessWindowStyle.Hidden
        });
    }
}

internal sealed class InstallForm : Form
{
    private readonly TextBox _directoryBox = new();
    private readonly CheckBox _startAfterInstall = new();

    public string InstallDirectory => _directoryBox.Text.Trim();
    public bool StartAfterInstall => _startAfterInstall.Checked;

    public InstallForm(string installDirectory, bool startAfterInstall)
    {
        Text = "TaskbarStats Setup";
        StartPosition = FormStartPosition.CenterScreen;
        FormBorderStyle = FormBorderStyle.FixedDialog;
        MaximizeBox = false;
        MinimizeBox = false;
        ClientSize = new Size(520, 260);
        Font = new Font("Segoe UI", 9F);

        var title = new Label
        {
            Text = "Install TaskbarStats",
            AutoSize = true,
            Font = new Font(Font.FontFamily, 18F, FontStyle.Regular),
            Location = new Point(24, 22)
        };
        Controls.Add(title);

        var subtitle = new Label
        {
            Text = "Choose where TaskbarStats will be installed.",
            AutoSize = true,
            ForeColor = SystemColors.GrayText,
            Location = new Point(27, 62)
        };
        Controls.Add(subtitle);

        var folderLabel = new Label
        {
            Text = "Install location",
            AutoSize = true,
            Location = new Point(27, 105)
        };
        Controls.Add(folderLabel);

        _directoryBox.Text = installDirectory;
        _directoryBox.Location = new Point(30, 128);
        _directoryBox.Size = new Size(370, 27);
        Controls.Add(_directoryBox);

        var browse = new Button
        {
            Text = "Browse...",
            Location = new Point(410, 127),
            Size = new Size(82, 29)
        };
        browse.Click += (_, _) => BrowseForFolder();
        Controls.Add(browse);

        _startAfterInstall.Text = "Start TaskbarStats after installation";
        _startAfterInstall.Checked = startAfterInstall;
        _startAfterInstall.AutoSize = true;
        _startAfterInstall.Location = new Point(30, 174);
        Controls.Add(_startAfterInstall);

        var install = new Button
        {
            Text = "Install",
            DialogResult = DialogResult.OK,
            Location = new Point(310, 214),
            Size = new Size(86, 30)
        };
        install.Click += (_, e) =>
        {
            if (string.IsNullOrWhiteSpace(InstallDirectory))
            {
                MessageBox.Show(
                    this,
                    "Please choose an install location.",
                    "TaskbarStats Setup",
                    MessageBoxButtons.OK,
                    MessageBoxIcon.Warning);
                DialogResult = DialogResult.None;
                return;
            }

            try
            {
                _directoryBox.Text = Path.GetFullPath(InstallDirectory);
            }
            catch (Exception ex)
            {
                MessageBox.Show(
                    this,
                    ex.Message,
                    "Invalid install location",
                    MessageBoxButtons.OK,
                    MessageBoxIcon.Warning);
                DialogResult = DialogResult.None;
            }
        };
        Controls.Add(install);

        var cancel = new Button
        {
            Text = "Cancel",
            DialogResult = DialogResult.Cancel,
            Location = new Point(406, 214),
            Size = new Size(86, 30)
        };
        Controls.Add(cancel);

        AcceptButton = install;
        CancelButton = cancel;
    }

    private void BrowseForFolder()
    {
        using var dialog = new FolderBrowserDialog
        {
            Description = "Choose TaskbarStats install location",
            UseDescriptionForTitle = true,
            SelectedPath = Directory.Exists(InstallDirectory)
                ? InstallDirectory
                : Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData)
        };

        if (dialog.ShowDialog(this) == DialogResult.OK)
        {
            string selected = dialog.SelectedPath;
            if (!selected.EndsWith("TaskbarStats", StringComparison.OrdinalIgnoreCase))
            {
                selected = Path.Combine(selected, "TaskbarStats");
            }
            _directoryBox.Text = selected;
        }
    }
}
