using System;
using System.Runtime.InteropServices;

namespace FlowReveal.Core.Capture
{
    // WFP 相关常量
    internal class WfpConstants
    {
        // 过滤层
        public const uint FWPM_LAYER_ALE_AUTH_CONNECT_V4 = 36;
        public const uint FWPM_LAYER_ALE_AUTH_RECV_ACCEPT_V4 = 37;
        public const uint FWPM_LAYER_STREAM_V4 = 40;
        public const uint FWPM_LAYER_INBOUND_TRANSPORT_V4 = 44;
        public const uint FWPM_LAYER_OUTBOUND_TRANSPORT_V4 = 45;
        
        // 会话标志
        public const uint FWPM_SESSION_FLAG_DYNAMIC = 0x00000001;
        
        // 过滤动作
        public const uint FWP_ACTION_BLOCK = 0x00000001;
        public const uint FWP_ACTION_PERMIT = 0x00000002;
        public const uint FWP_ACTION_CALLOUT_TERMINATING = 0x00000003;
        
        // 过滤条件类型
        public const uint FWP_CONDITION_TYPE_UINT32 = 0x00000006;
        public const uint FWP_CONDITION_TYPE_IPV4_ADDRESS = 0x00000007;
        public const uint FWP_CONDITION_TYPE_UINT16 = 0x00000008;
    }

    // WFP 会话结构
    [StructLayout(LayoutKind.Sequential)]
    internal struct FWPM_SESSION0
    {
        public IntPtr sessionKey;
        public uint flags;
        public IntPtr displayData;
        public IntPtr providerKey;
    }

    // WFP 过滤器结构
    [StructLayout(LayoutKind.Sequential)]
    internal struct FWPM_FILTER0
    {
        public IntPtr filterKey;
        public uint displayData;
        public uint flags;
        public uint layerKey;
        public IntPtr subLayerKey;
        public IntPtr weight;
        public IntPtr filterCondition;
        public uint numFilterConditions;
        public uint actionType;
        public IntPtr action;
        public IntPtr providerKey;
        public uint providerData;
        public uint providerContextKey;
    }

    // WFP 条件结构
    [StructLayout(LayoutKind.Sequential)]
    internal struct FWPM_FILTER_CONDITION0
    {
        public uint fieldKey;
        public uint conditionType;
        public IntPtr conditionValue;
    }

    // WFP 回调函数委托
    internal delegate void WfpPacketCallback(IntPtr packetData, IntPtr context);

    // WFP API 导入
    internal class WfpApi
    {
        [DllImport("fwpuclnt.dll", SetLastError = true, CharSet = CharSet.Unicode)]
        public static extern uint FwpmEngineOpen0(
            [MarshalAs(UnmanagedType.LPWStr)] string serverName,
            uint authnService,
            IntPtr authIdentity,
            IntPtr session,
            out IntPtr engineHandle
        );

        [DllImport("fwpuclnt.dll", SetLastError = true)]
        public static extern uint FwpmEngineClose0(
            IntPtr engineHandle
        );

        [DllImport("fwpuclnt.dll", SetLastError = true)]
        public static extern uint FwpmFilterAdd0(
            IntPtr engineHandle,
            ref FWPM_FILTER0 filter,
            IntPtr sd,
            out IntPtr filterId
        );

        [DllImport("fwpuclnt.dll", SetLastError = true)]
        public static extern uint FwpmFilterDeleteById0(
            IntPtr engineHandle,
            IntPtr filterId
        );

        [DllImport("fwpuclnt.dll", SetLastError = true)]
        public static extern uint FwpmSessionCreate0(
            ref FWPM_SESSION0 session,
            out IntPtr sessionHandle
        );

        [DllImport("fwpuclnt.dll", SetLastError = true)]
        public static extern uint FwpmSessionDestroy0(
            IntPtr sessionHandle
        );
    }
}