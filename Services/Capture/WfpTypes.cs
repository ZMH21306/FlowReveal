using System;
using System.Runtime.InteropServices;

namespace FlowReveal.Services.Capture;

[StructLayout(LayoutKind.Sequential)]
public struct FWPM_FILTER0
{
    public IntPtr providerKey;
    public Guid filterKey;
    public UInt64 weight;
    public UInt32 flags;
    public IntPtr name;
    public IntPtr description;
    public Guid filterId;
    public Guid subLayerKey;
    public Guid calloutKey;
    public UInt64 flowId;
    public IntPtr sessionKey;
    public UInt32 numFilterConditions;
    public FWPM_FILTER_CONDITION0[] filterCondition;
    public IntPtr action;
    public FWPM_PROVIDER_CONTEXT0[] providerContext;
}

[StructLayout(LayoutKind.Sequential)]
public struct FWPM_FILTER_CONDITION0
{
    public Guid fieldKey;
    public UInt16 matchType;
    public IntPtr conditionValue;
    public UInt32 conditionValueSize;
}

[StructLayout(LayoutKind.Sequential)]
public struct FWPM_PROVIDER_CONTEXT0
{
    public Guid providerContextKey;
    public IntPtr providerKey;
    public UInt32 type;
    public IntPtr data;
    public UInt32 dataSize;
    public UInt32 flags;
    public IntPtr name;
    public IntPtr description;
    public Guid sessionKey;
}

[StructLayout(LayoutKind.Sequential)]
public struct FWPM_SESSION0
{
    public UInt32 flags;
    public IntPtr providerKey;
    public IntPtr name;
}

[StructLayout(LayoutKind.Sequential)]
public struct FWPM_PROVIDER0
{
    public Guid providerKey;
    public IntPtr name;
    public IntPtr description;
    public UInt32 flags;
}

[StructLayout(LayoutKind.Sequential)]
public struct FWPM_SUBLAYER0
{
    public Guid subLayerKey;
    public Guid providerKey;
    public IntPtr name;
    public IntPtr description;
    public UInt32 flags;
    public UInt16 weight;
}

[StructLayout(LayoutKind.Sequential)]
public struct FWP_VALUE0
{
    public UInt16 type;
    public FWP_VALUE_UNION value;
}

[StructLayout(LayoutKind.Explicit)]
public struct FWP_VALUE_UNION
{
    [FieldOffset(0)] public UInt32 uint32;
    [FieldOffset(0)] public UInt64 uint64;
    [FieldOffset(0)] public IntPtr byteBlob;
    [FieldOffset(0)] public IntPtr @string;
    [FieldOffset(0)] public Guid guid;
}

[StructLayout(LayoutKind.Sequential)]
public struct FWP_BYTE_BLOB
{
    public UInt32 size;
    public IntPtr data;
}

[StructLayout(LayoutKind.Sequential)]
public struct FWPM_CALLOUT0
{
    public Guid calloutKey;
    public Guid providerKey;
    public UInt32 flags;
    public IntPtr name;
    public IntPtr description;
    public IntPtr classifyFn;
    public IntPtr notifyFn;
    public UInt32 numFilterConditions;
    public FWPM_FILTER_CONDITION0[] filterCondition;
}

public static class FwpmConstants
{
    public static readonly Guid FWPM_LAYER_ALE_AUTH_CONNECT_V4 = new("3FE8959C-D555-48D9-95C8-4E3C71F07174");
    public static readonly Guid FWPM_LAYER_ALE_AUTH_CONNECT_V6 = new("E4CAF12A-A735-4D43-BBE9-9DAB4B565BD4");
    
    public static readonly Guid FWPM_CONDITION_IP_LOCAL_PORT = new("5411AB6B-6ED4-4EC8-A01D-438571BA1409");
    public static readonly Guid FWPM_CONDITION_IP_REMOTE_PORT = new("44806EA8-8A05-4D8C-9A3B-16A0EE3A9844");
    public static readonly Guid FWPM_CONDITION_IP_PROTOCOL = new("AF9627E1-8E05-4D9A-A46D-0073C956D778");
    
    public static readonly Guid FWPM_ACTION_BLOCK = new("8145E391-45E9-4351-A593-5B8EC27E0468");
    public static readonly Guid FWPM_ACTION_PERMIT = new("6EDD595B-08B0-418D-805E-2D6D7C86D86A");
    public static readonly Guid FWPM_ACTION_CALLOUT_TERMINATING = new("2EBF46B3-87F9-4020-A47D-602A47D35D6D");
    
    public const uint FWPM_FILTER_FLAG_PERSISTENT = 0x00000001;
    public const uint FWPM_FILTER_FLAG_BOOTTIME = 0x00000002;
    public const uint FWPM_FILTER_FLAG_DISABLED = 0x00000004;
    
    public const ushort FWP_MATCH_EQUAL = 0x0000;
    public const ushort FWP_MATCH_GREATER_OR_EQUAL = 0x0003;
    public const ushort FWP_MATCH_LESS_OR_EQUAL = 0x0004;
    
    public const ushort FWP_UINT32 = 0x0001;
    public const ushort FWP_UINT64 = 0x0002;
    public const ushort FWP_BYTE_BLOB_TYPE = 0x0003;
    public const ushort FWP_STRING = 0x0004;
    public const ushort FWP_GUID = 0x0005;
    
    public const uint AF_INET = 2;
    public const uint AF_INET6 = 23;
    public const uint IPPROTO_TCP = 6;
    
    public static readonly Guid GUID_NULL = Guid.Empty;
}