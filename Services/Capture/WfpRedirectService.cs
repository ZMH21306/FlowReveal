using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;

namespace FlowReveal.Services.Capture;

public class WfpRedirectService : IDisposable
{
    private IntPtr _engineHandle = IntPtr.Zero;
    private Guid _providerKey = new Guid("F1B54E0E-7E9A-4D2A-8C5B-7D8E9F0A1B2C");
    private Guid _subLayerKey = new Guid("F1B54E0E-7E9A-4D2A-8C5B-7D8E9F0A1B2D");
    private readonly List<ulong> _filterIds = new();
    private bool _isStarted;
    private readonly int _httpPort;
    private readonly int _httpsPort;

    public WfpRedirectService(int httpPort = 9080, int httpsPort = 9443)
    {
        _httpPort = httpPort;
        _httpsPort = httpsPort;
    }

    public bool Start()
    {
        if (_isStarted)
            return true;

        try
        {
            uint result = WfpInterop.FwpmEngineOpen(out _engineHandle);
            if (result != 0)
                return false;

            result = WfpInterop.FwpmTransactionBegin(_engineHandle);
            if (result != 0)
                return false;

            if (!AddProvider())
                return false;

            if (!AddSubLayer())
                return false;

            if (!AddFilters())
                return false;

            result = WfpInterop.FwpmTransactionCommit(_engineHandle);
            if (result != 0)
                return false;

            _isStarted = true;
            return true;
        }
        catch
        {
            Cleanup();
            return false;
        }
    }

    public bool Stop()
    {
        if (!_isStarted)
            return true;

        try
        {
            Cleanup();
            _isStarted = false;
            return true;
        }
        catch
        {
            return false;
        }
    }

    private bool AddProvider()
    {
        FWPM_PROVIDER0 provider = new();
        Guid key = _providerKey;
        provider.providerKey = key;
        provider.name = Marshal.StringToHGlobalUni("FlowReveal Provider");
        provider.description = Marshal.StringToHGlobalUni("FlowReveal HTTP Debugger Traffic Redirect");
        provider.flags = 0;

        uint result = WfpInterop.FwpmProviderAdd(_engineHandle, ref provider, IntPtr.Zero);

        Marshal.FreeHGlobal(provider.name);
        Marshal.FreeHGlobal(provider.description);

        return result == 0;
    }

    private bool AddSubLayer()
    {
        FWPM_SUBLAYER0 subLayer = new();
        Guid subKey = _subLayerKey;
        Guid provKey = _providerKey;
        subLayer.subLayerKey = subKey;
        subLayer.providerKey = provKey;
        subLayer.name = Marshal.StringToHGlobalUni("FlowReveal SubLayer");
        subLayer.description = Marshal.StringToHGlobalUni("FlowReveal HTTP Debugger SubLayer");
        subLayer.flags = 0;
        subLayer.weight = 0xFFFF;

        uint result = WfpInterop.FwpmSubLayerAdd(_engineHandle, ref subLayer, IntPtr.Zero);

        Marshal.FreeHGlobal(subLayer.name);
        Marshal.FreeHGlobal(subLayer.description);

        return result == 0;
    }

    private bool AddFilters()
    {
        bool httpResult = AddRedirectFilter(_httpPort, "HTTP Redirect");
        bool httpsResult = AddRedirectFilter(_httpsPort, "HTTPS Redirect");

        return httpResult && httpsResult;
    }

    private bool AddRedirectFilter(int port, string name)
    {
        try
        {
            FWPM_FILTER0 filter = new();
            filter.filterKey = Guid.NewGuid();
            
            Guid subKey = _subLayerKey;
            filter.subLayerKey = subKey;
            
            filter.providerKey = IntPtr.Zero;
            filter.name = Marshal.StringToHGlobalUni(name);
            filter.description = Marshal.StringToHGlobalUni($"Redirect traffic to port {port}");
            filter.flags = 0;
            filter.weight = 0x8000000000000000UL;

            FWPM_FILTER_CONDITION0[] conditions = new FWPM_FILTER_CONDITION0[2];
            
            conditions[0].fieldKey = FwpmConstants.FWPM_CONDITION_IP_PROTOCOL;
            conditions[0].matchType = FwpmConstants.FWP_MATCH_EQUAL;
            conditions[0].conditionValueSize = 4;
            FWP_VALUE0 protoValue = new();
            protoValue.type = FwpmConstants.FWP_UINT32;
            protoValue.value.uint32 = FwpmConstants.IPPROTO_TCP;
            conditions[0].conditionValue = Marshal.AllocHGlobal(Marshal.SizeOf(protoValue));
            Marshal.StructureToPtr(protoValue, conditions[0].conditionValue, false);

            conditions[1].fieldKey = FwpmConstants.FWPM_CONDITION_IP_REMOTE_PORT;
            conditions[1].matchType = FwpmConstants.FWP_MATCH_EQUAL;
            conditions[1].conditionValueSize = 4;
            FWP_VALUE0 portValue = new();
            portValue.type = FwpmConstants.FWP_UINT32;
            portValue.value.uint32 = (uint)port;
            conditions[1].conditionValue = Marshal.AllocHGlobal(Marshal.SizeOf(portValue));
            Marshal.StructureToPtr(portValue, conditions[1].conditionValue, false);

            filter.filterCondition = conditions;
            filter.numFilterConditions = (uint)conditions.Length;

            FWP_VALUE0 actionValue = new();
            actionValue.type = FwpmConstants.FWP_GUID;
            actionValue.value.guid = FwpmConstants.FWPM_ACTION_PERMIT;
            IntPtr actionPtr = Marshal.AllocHGlobal(Marshal.SizeOf(actionValue));
            Marshal.StructureToPtr(actionValue, actionPtr, false);
            filter.action = actionPtr;

            ulong filterId;
            uint result = WfpInterop.FwpmFilterAdd(_engineHandle, ref filter, IntPtr.Zero, out filterId);
            _filterIds.Add(filterId);

            Marshal.FreeHGlobal(filter.name);
            Marshal.FreeHGlobal(filter.description);
            Marshal.FreeHGlobal(conditions[0].conditionValue);
            Marshal.FreeHGlobal(conditions[1].conditionValue);
            Marshal.FreeHGlobal(actionPtr);

            return result == 0;
        }
        catch
        {
            return false;
        }
    }

    private void Cleanup()
    {
        if (_engineHandle != IntPtr.Zero)
        {
            foreach (ulong filterId in _filterIds)
            {
                WfpInterop.FwpmFilterDeleteById(_engineHandle, filterId);
            }
            _filterIds.Clear();

            Guid subKey = _subLayerKey;
            WfpInterop.FwpmSubLayerDeleteByKey(_engineHandle, ref subKey);
            
            Guid provKey = _providerKey;
            WfpInterop.FwpmProviderDeleteByKey(_engineHandle, ref provKey);
            
            WfpInterop.FwpmEngineClose(_engineHandle);
            _engineHandle = IntPtr.Zero;
        }
    }

    public void Dispose()
    {
        Stop();
    }

    public bool IsStarted => _isStarted;
}