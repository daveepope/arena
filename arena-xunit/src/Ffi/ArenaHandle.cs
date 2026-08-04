using System;
using Microsoft.Win32.SafeHandles;

namespace ArenaDotnet.Xunit.Ffi;

internal sealed class ArenaHandle : SafeHandleZeroOrMinusOneIsInvalid
{
    public ArenaHandle(IntPtr handle) : base(true)
    {
        SetHandle(handle);
    }

    protected override bool ReleaseHandle()
    {
        try
        {
            ArenaBindings.CloseArena(handle);
            return true;
        }
        catch
        {
            return false;
        }
    }
}

internal sealed class ActivePlaybookHandle : SafeHandleZeroOrMinusOneIsInvalid
{
    public ActivePlaybookHandle(IntPtr handle) : base(true)
    {
        SetHandle(handle);
    }

    protected override bool ReleaseHandle()
    {
        try
        {
            ArenaBindings.ActivePlaybookDrop(handle);
            return true;
        }
        catch
        {
            return false;
        }
    }
}
