// Compiler support types that netstandard2.0 lacks. `init` accessors (and so
// positional records) need IsExternalInit; the compiler finds it by name.

#if !NET5_0_OR_GREATER
namespace System.Runtime.CompilerServices
{
    internal static class IsExternalInit
    {
    }
}
#endif
