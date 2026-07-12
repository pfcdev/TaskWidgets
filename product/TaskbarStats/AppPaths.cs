namespace TaskbarStatsProduct;

internal static class AppPaths
{
    public static string AppDirectory { get; } = ResolveAppDirectory();

    private static string ResolveAppDirectory()
    {
        var processPath = Environment.ProcessPath;
        if (!string.IsNullOrWhiteSpace(processPath))
        {
            var directory = Path.GetDirectoryName(Path.GetFullPath(processPath));
            if (!string.IsNullOrWhiteSpace(directory))
            {
                return directory;
            }
        }

        return AppContext.BaseDirectory.TrimEnd(
            Path.DirectorySeparatorChar,
            Path.AltDirectorySeparatorChar);
    }
}
