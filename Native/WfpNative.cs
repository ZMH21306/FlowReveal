using System;
using System.Runtime.InteropServices;

namespace FlowReveal.Native
{
    public static class WfpNative
    {
        private const string FwpuclntDll = "fwpuclnt.dll";

        [DllImport(FwpuclntDll, CallingConvention = CallingConvention.Winapi)]
        public static extern int FwpmEngineOpen0(
            [MarshalAs(UnmanagedType.LPWStr)] string serverName,
            int authnService,
            IntPtr authIdentity,
            IntPtr engineOptions,
            out IntPtr engineHandle
        );

        [DllImport(FwpuclntDll, CallingConvention = CallingConvention.Winapi)]
        public static extern int FwpmEngineClose0(
            IntPtr engineHandle
        );

        [DllImport(FwpuclntDll, CallingConvention = CallingConvention.Winapi)]
        public static extern int FwpmSubLayerAdd0(
            IntPtr engineHandle,
            ref FWPM_SUBLAYER0 subLayer,
            IntPtr sd
        );

        [DllImport(FwpuclntDll, CallingConvention = CallingConvention.Winapi)]
        public static extern int FwpmFilterAdd0(
            IntPtr engineHandle,
            ref FWPM_FILTER0 filter,
            IntPtr sd,
            out ulong filterId
        );

        [DllImport(FwpuclntDll, CallingConvention = CallingConvention.Winapi)]
        public static extern int FwpmFreeMemory0(
            ref IntPtr memory
        );

        [StructLayout(LayoutKind.Sequential)]
        public struct FWPM_SUBLAYER0
        {
            public Guid subLayerKey;
            public uint flags;
            public int weight;
            [MarshalAs(UnmanagedType.LPWStr)]
            public string displayDataName;
            [MarshalAs(UnmanagedType.LPWStr)]
            public string displayDataDescription;
        }

        [StructLayout(LayoutKind.Sequential)]
        public struct FWPM_FILTER0
        {
            public ulong filterId;
            public Guid filterKey;
            public FWPM_SUBLAYER0 subLayerKey;
            public uint flags;
            public int weight;
            public IntPtr condition;
            public uint numConditions;
            public IntPtr action;
            [MarshalAs(UnmanagedType.LPWStr)]
            public string displayDataName;
            [MarshalAs(UnmanagedType.LPWStr)]
            public string displayDataDescription;
        }
    }
}
