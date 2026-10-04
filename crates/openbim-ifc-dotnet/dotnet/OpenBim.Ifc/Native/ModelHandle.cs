using System;
using System.Runtime.InteropServices;

namespace OpenBim.Ifc.Native;

/// <summary>
/// The C ABI's opaque model handle (<c>OpenbimIfcModel</c>, a non-zero
/// integer), owned by a <see cref="SafeHandle"/>: the model is destroyed
/// exactly once, by <see cref="IfcModel.Dispose"/> or by the finalizer, and
/// never while a call on it is running.
/// </summary>
/// <remarks>
/// The handle is a 64-bit integer stored in an <see cref="IntPtr"/>, so it
/// needs a 64-bit process; the package ships 64-bit libraries only.
/// </remarks>
internal sealed class ModelHandle : SafeHandle
{
    private ModelHandle()
        : base(IntPtr.Zero, ownsHandle: true)
    {
    }

    /// <summary>Take ownership of a handle the C ABI returned.</summary>
    internal static ModelHandle Own(ulong value)
    {
        var handle = new ModelHandle();
        handle.SetHandle(new IntPtr(unchecked((long)value)));
        return handle;
    }

    /// <inheritdoc/>
    public override bool IsInvalid => handle == IntPtr.Zero;

    /// <summary>The raw handle value; valid while a <see cref="Lease"/> is held.</summary>
    internal ulong Value => unchecked((ulong)handle.ToInt64());

    /// <summary>Keep the handle alive for one native call.</summary>
    internal Lease Acquire()
    {
        var added = false;
        DangerousAddRef(ref added);
        return new Lease(this, added);
    }

    /// <inheritdoc/>
    protected override bool ReleaseHandle() =>
        NativeMethods.openbim_ifc_v0_1_model_destroy(unchecked((ulong)handle.ToInt64())) == IfcStatus.Ok;

    /// <summary>A reference on the handle for the duration of a call.</summary>
    internal readonly struct Lease : IDisposable
    {
        private readonly ModelHandle owner;
        private readonly bool added;

        internal Lease(ModelHandle owner, bool added)
        {
            this.owner = owner;
            this.added = added;
        }

        /// <summary>The handle value to pass to the C ABI.</summary>
        internal ulong Model => owner.Value;

        public void Dispose()
        {
            if (added)
            {
                owner.DangerousRelease();
            }
        }
    }
}
