// Finding the C ABI's shared library.
//
// A .NET (Core) application resolves runtimes/<rid>/native/ from its
// deps.json, and a project built for one RID gets the file next to its
// assembly; both need nothing from here. Two hosts do:
//
// - a plug-in loaded into a host that does not read the plug-in's deps.json
//   (an add-in of a .NET 8 application, loaded by path);
// - a .NET Framework add-in (Revit, Navisworks, Tekla), where DllImport
//   searches the host's directory and PATH, never the add-in's. The package's
//   build targets copy runtimes/win-<arch>/native/ into its output.
//
// For both, the library is looked up next to this assembly, first under
// runtimes/<rid>/native/, then beside it. The default search runs first, so
// an application that ships the library its own way keeps it.

using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;

namespace OpenBim.Ifc.Native;

internal static class NativeLibraryLoader
{
    /// <summary>The runtime identifier of this process, e.g. <c>linux-x64</c>,
    /// or null on a platform the package ships no library for.</summary>
    internal static string? RuntimeIdentifier
    {
        get
        {
            string os;
            if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            {
                os = "win";
            }
            else if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
            {
                os = "osx";
            }
            else if (RuntimeInformation.IsOSPlatform(OSPlatform.Linux))
            {
                os = "linux";
            }
            else
            {
                return null;
            }
            return RuntimeInformation.ProcessArchitecture switch
            {
                Architecture.X64 => os + "-x64",
                Architecture.Arm64 => os + "-arm64",
                _ => null,
            };
        }
    }

    /// <summary>The library's file name on this platform.</summary>
    internal static string FileName =>
        RuntimeInformation.IsOSPlatform(OSPlatform.Windows) ? NativeMethods.Library + ".dll"
        : RuntimeInformation.IsOSPlatform(OSPlatform.OSX) ? "lib" + NativeMethods.Library + ".dylib"
        : "lib" + NativeMethods.Library + ".so";

    /// <summary>Candidate paths next to this assembly, most specific first.</summary>
    internal static List<string> Candidates()
    {
        var rid = RuntimeIdentifier;
        var candidates = new List<string>();
        foreach (var directory in Directories())
        {
            if (rid is not null)
            {
                candidates.Add(Path.Combine(directory, "runtimes", rid, "native", FileName));
            }
            candidates.Add(Path.Combine(directory, FileName));
        }
        return candidates;
    }

    /// <summary>Where this assembly was loaded from, where it was installed
    /// (a shadow-copying host such as a .NET Framework test runner loads a
    /// copy from elsewhere), and the application's base directory.</summary>
    private static IEnumerable<string> Directories()
    {
        var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        var assembly = typeof(NativeLibraryLoader).Assembly;
        var paths = new List<string?> { assembly.Location };
#if !NET5_0_OR_GREATER
        try
        {
            var codeBase = assembly.CodeBase;
            if (!string.IsNullOrEmpty(codeBase) && Uri.TryCreate(codeBase, UriKind.Absolute, out var uri) && uri.IsFile)
            {
                paths.Add(uri.LocalPath);
            }
        }
        catch (NotSupportedException)
        {
            // A dynamic or in-memory assembly has no code base.
        }
#endif
        foreach (var path in paths)
        {
            var directory = string.IsNullOrEmpty(path) ? null : Path.GetDirectoryName(path);
            if (!string.IsNullOrEmpty(directory) && seen.Add(directory!))
            {
                yield return directory!;
            }
        }
        var baseDirectory = AppDomain.CurrentDomain.BaseDirectory;
        if (!string.IsNullOrEmpty(baseDirectory) && seen.Add(baseDirectory.TrimEnd(Path.DirectorySeparatorChar)))
        {
            yield return baseDirectory;
        }
    }

#if NET5_0_OR_GREATER
    /// <summary>Install the fallback resolver for this assembly's imports.
    /// Runs once, from <see cref="NativeMethods"/>'s type initializer.</summary>
    internal static void Register()
    {
        try
        {
            NativeLibrary.SetDllImportResolver(typeof(NativeLibraryLoader).Assembly, Resolve);
        }
        catch (InvalidOperationException)
        {
            // The host installed a resolver for this assembly already; it wins.
        }
    }

    private static IntPtr Resolve(string name, System.Reflection.Assembly assembly, DllImportSearchPath? searchPath)
    {
        if (name != NativeMethods.Library)
        {
            return IntPtr.Zero;
        }
        if (NativeLibrary.TryLoad(name, assembly, searchPath, out var handle))
        {
            return handle;
        }
        foreach (var candidate in Candidates())
        {
            if (File.Exists(candidate) && NativeLibrary.TryLoad(candidate, out handle))
            {
                return handle;
            }
        }
        return IntPtr.Zero;
    }
#else
    /// <summary>On Windows, load the library from next to this assembly before
    /// the first import binds, so the import finds the loaded module by name.
    /// Elsewhere the runtime's own search applies.</summary>
    internal static void Register()
    {
        if (!RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
        {
            return;
        }
        foreach (var candidate in Candidates())
        {
            if (File.Exists(candidate) && LoadLibraryExW(candidate, IntPtr.Zero, LoadWithAlteredSearchPath) != IntPtr.Zero)
            {
                return;
            }
        }
    }

    private const uint LoadWithAlteredSearchPath = 0x00000008;

    [DllImport("kernel32", CharSet = CharSet.Unicode, ExactSpelling = true, SetLastError = true)]
    private static extern IntPtr LoadLibraryExW(string fileName, IntPtr file, uint flags);
#endif
}
