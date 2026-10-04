using OpenBim.Ifc.Native;

namespace OpenBim.Ifc;

/// <summary>The loaded native library: its versions and its live models.</summary>
public static unsafe class IfcLibrary
{
    /// <summary>The C ABI and library versions, reported separately: the ABI
    /// changes only when the C surface does, the library on every release.</summary>
    public static LibraryVersion Version
    {
        get
        {
            NativeVersion version;
            var status = NativeMethods.openbim_ifc_v0_1_version(&version);
            if (status != IfcStatus.Ok)
            {
                throw Calls.Error(status, null);
            }
            return new LibraryVersion(
                new System.Version(version.AbiMajor, version.AbiMinor, version.AbiPatch),
                new System.Version(version.CrateMajor, version.CrateMinor, version.CratePatch));
        }
    }

    /// <summary>Models the library holds: created and not yet disposed or
    /// finalized. For leak checks.</summary>
    public static long LiveModels
    {
        get
        {
            nuint count = 0;
            var status = NativeMethods.openbim_ifc_v0_1_live_models(&count);
            if (status != IfcStatus.Ok)
            {
                throw Calls.Error(status, null);
            }
            return (long)count;
        }
    }
}
