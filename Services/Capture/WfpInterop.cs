using System;
using System.Runtime.InteropServices;

namespace FlowReveal.Services.Capture;

public static class WfpInterop
{
    private const string FwpuclntDll = "fwpuclnt.dll";

    [DllImport(FwpuclntDll, CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern uint FwpmEngineOpen0(
        IntPtr serverName,
        uint flags,
        IntPtr authData,
        IntPtr session,
        out IntPtr engineHandle
    );

    [DllImport(FwpuclntDll, SetLastError = true)]
    public static extern uint FwpmEngineClose0(
        IntPtr engineHandle
    );

    [DllImport(FwpuclntDll, CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern uint FwpmProviderAdd0(
        IntPtr engineHandle,
        [In] ref FWPM_PROVIDER0 provider,
        IntPtr sd
    );

    [DllImport(FwpuclntDll, CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern uint FwpmProviderDeleteByKey0(
        IntPtr engineHandle,
        [In] ref Guid providerKey
    );

    [DllImport(FwpuclntDll, CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern uint FwpmSubLayerAdd0(
        IntPtr engineHandle,
        [In] ref FWPM_SUBLAYER0 subLayer,
        IntPtr sd
    );

    [DllImport(FwpuclntDll, SetLastError = true)]
    public static extern uint FwpmSubLayerDeleteByKey0(
        IntPtr engineHandle,
        [In] ref Guid subLayerKey
    );

    [DllImport(FwpuclntDll, CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern uint FwpmFilterAdd0(
        IntPtr engineHandle,
        [In] ref FWPM_FILTER0 filter,
        IntPtr sd,
        out ulong filterId
    );

    [DllImport(FwpuclntDll, SetLastError = true)]
    public static extern uint FwpmFilterDeleteByKey0(
        IntPtr engineHandle,
        [In] ref Guid filterKey
    );

    [DllImport(FwpuclntDll, SetLastError = true)]
    public static extern uint FwpmFilterDeleteById0(
        IntPtr engineHandle,
        ulong filterId
    );

    [DllImport(FwpuclntDll, CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern uint FwpmTransactionBegin0(
        IntPtr engineHandle
    );

    [DllImport(FwpuclntDll, SetLastError = true)]
    public static extern uint FwpmTransactionCommit0(
        IntPtr engineHandle
    );

    [DllImport(FwpuclntDll, SetLastError = true)]
    public static extern uint FwpmTransactionAbort0(
        IntPtr engineHandle
    );

    [DllImport(FwpuclntDll, SetLastError = true)]
    public static extern uint FwpmCalloutAdd0(
        IntPtr engineHandle,
        [In] ref FWPM_CALLOUT0 callout,
        IntPtr sd,
        out ulong calloutId
    );

    [DllImport(FwpuclntDll, SetLastError = true)]
    public static extern uint FwpmCalloutDeleteByKey0(
        IntPtr engineHandle,
        [In] ref Guid calloutKey
    );

    [DllImport(FwpuclntDll, SetLastError = true)]
    public static extern uint FwpmFreeMemory0(
        ref IntPtr p
    );

    public static uint FwpmEngineOpen(out IntPtr engineHandle)
    {
        return FwpmEngineOpen0(IntPtr.Zero, 0, IntPtr.Zero, IntPtr.Zero, out engineHandle);
    }

    public static uint FwpmEngineClose(IntPtr engineHandle)
    {
        return FwpmEngineClose0(engineHandle);
    }

    public static uint FwpmProviderAdd(IntPtr engineHandle, ref FWPM_PROVIDER0 provider, IntPtr sd)
    {
        return FwpmProviderAdd0(engineHandle, ref provider, sd);
    }

    public static uint FwpmProviderDeleteByKey(IntPtr engineHandle, ref Guid providerKey)
    {
        return FwpmProviderDeleteByKey0(engineHandle, ref providerKey);
    }

    public static uint FwpmSubLayerAdd(IntPtr engineHandle, ref FWPM_SUBLAYER0 subLayer, IntPtr sd)
    {
        return FwpmSubLayerAdd0(engineHandle, ref subLayer, sd);
    }

    public static uint FwpmSubLayerDeleteByKey(IntPtr engineHandle, ref Guid subLayerKey)
    {
        return FwpmSubLayerDeleteByKey0(engineHandle, ref subLayerKey);
    }

    public static uint FwpmFilterAdd(IntPtr engineHandle, ref FWPM_FILTER0 filter, IntPtr sd, out ulong filterId)
    {
        return FwpmFilterAdd0(engineHandle, ref filter, sd, out filterId);
    }

    public static uint FwpmFilterDeleteByKey(IntPtr engineHandle, ref Guid filterKey)
    {
        return FwpmFilterDeleteByKey0(engineHandle, ref filterKey);
    }

    public static uint FwpmFilterDeleteById(IntPtr engineHandle, ulong filterId)
    {
        return FwpmFilterDeleteById0(engineHandle, filterId);
    }

    public static uint FwpmTransactionBegin(IntPtr engineHandle)
    {
        return FwpmTransactionBegin0(engineHandle);
    }

    public static uint FwpmTransactionCommit(IntPtr engineHandle)
    {
        return FwpmTransactionCommit0(engineHandle);
    }

    public static uint FwpmTransactionAbort(IntPtr engineHandle)
    {
        return FwpmTransactionAbort0(engineHandle);
    }

    public static uint FwpmCalloutAdd(IntPtr engineHandle, ref FWPM_CALLOUT0 callout, IntPtr sd, out ulong calloutId)
    {
        return FwpmCalloutAdd0(engineHandle, ref callout, sd, out calloutId);
    }

    public static uint FwpmCalloutDeleteByKey(IntPtr engineHandle, ref Guid calloutKey)
    {
        return FwpmCalloutDeleteByKey0(engineHandle, ref calloutKey);
    }

    public static uint FwpmFreeMemory(ref IntPtr p)
    {
        return FwpmFreeMemory0(ref p);
    }
}