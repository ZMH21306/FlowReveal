using System;
using System.Runtime.InteropServices;

namespace FlowReveal.Services.Capture;

public static class WfpInterop
{
    private const string FwpuclntDll = "fwpuclnt.dll";

    [DllImport(FwpuclntDll, CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern uint FwpmEngineOpen(
        IntPtr serverName,
        uint flags,
        IntPtr authData,
        [In] ref FWPM_SESSION0 session,
        out IntPtr engineHandle
    );

    [DllImport(FwpuclntDll, SetLastError = true)]
    public static extern uint FwpmEngineClose(
        IntPtr engineHandle
    );

    [DllImport(FwpuclntDll, CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern uint FwpmProviderAdd(
        IntPtr engineHandle,
        [In] ref FWPM_PROVIDER0 provider,
        IntPtr sd
    );

    [DllImport(FwpuclntDll, CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern uint FwpmProviderDeleteByKey(
        IntPtr engineHandle,
        [In] ref Guid providerKey
    );

    [DllImport(FwpuclntDll, CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern uint FwpmSubLayerAdd(
        IntPtr engineHandle,
        [In] ref FWPM_SUBLAYER0 subLayer,
        IntPtr sd
    );

    [DllImport(FwpuclntDll, SetLastError = true)]
    public static extern uint FwpmSubLayerDeleteByKey(
        IntPtr engineHandle,
        [In] ref Guid subLayerKey
    );

    [DllImport(FwpuclntDll, CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern uint FwpmFilterAdd(
        IntPtr engineHandle,
        [In] ref FWPM_FILTER0 filter,
        IntPtr sd,
        out ulong filterId
    );

    [DllImport(FwpuclntDll, SetLastError = true)]
    public static extern uint FwpmFilterDeleteByKey(
        IntPtr engineHandle,
        [In] ref Guid filterKey
    );

    [DllImport(FwpuclntDll, SetLastError = true)]
    public static extern uint FwpmFilterDeleteById(
        IntPtr engineHandle,
        ulong filterId
    );

    [DllImport(FwpuclntDll, CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern uint FwpmTransactionBegin(
        IntPtr engineHandle
    );

    [DllImport(FwpuclntDll, SetLastError = true)]
    public static extern uint FwpmTransactionCommit(
        IntPtr engineHandle
    );

    [DllImport(FwpuclntDll, SetLastError = true)]
    public static extern uint FwpmTransactionAbort(
        IntPtr engineHandle
    );

    [DllImport(FwpuclntDll, SetLastError = true)]
    public static extern uint FwpmCalloutAdd(
        IntPtr engineHandle,
        [In] ref FWPM_CALLOUT0 callout,
        IntPtr sd,
        out ulong calloutId
    );

    [DllImport(FwpuclntDll, SetLastError = true)]
    public static extern uint FwpmCalloutDeleteByKey(
        IntPtr engineHandle,
        [In] ref Guid calloutKey
    );

    [DllImport(FwpuclntDll, SetLastError = true)]
    public static extern uint FwpmFreeMemory(
        ref IntPtr p
    );

    public static uint FwpmEngineOpen(out IntPtr engineHandle)
    {
        FWPM_SESSION0 session = new();
        return FwpmEngineOpen(IntPtr.Zero, 0, IntPtr.Zero, ref session, out engineHandle);
    }
}