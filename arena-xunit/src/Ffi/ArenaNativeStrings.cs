using System;
using System.Runtime.InteropServices;
using System.Text;

namespace ArenaXunit.Ffi;

internal static class ArenaNativeStrings
{
    public static string FromUtf8Ptr(IntPtr ptr)
    {
        if (ptr == IntPtr.Zero)
            return "";

        int len = 0;
        while (Marshal.ReadByte(ptr, len) != 0)
            len++;

        byte[] buffer = new byte[len];
        Marshal.Copy(ptr, buffer, 0, len);
        return Encoding.UTF8.GetString(buffer);
    }
}
