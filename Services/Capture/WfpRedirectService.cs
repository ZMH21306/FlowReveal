using FlowReveal.Services.Logging;
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
        Logger.LogInfo($"WfpRedirectService initialized with HTTP port {httpPort}, HTTPS port {httpsPort}");
    }

    public bool Start()
    {
        if (_isStarted)
        {
            Logger.LogWarning("WFP service is already running");
            return true;
        }

        try
        {
            Logger.LogInfo("Opening WFP engine handle");
            uint result = WfpInterop.FwpmEngineOpen(out _engineHandle);
            if (result != 0)
            {
                Logger.LogError($"Failed to open WFP engine. Error code: {result}");
                return false;
            }
            Logger.LogInfo("WFP engine opened successfully");

            Logger.LogInfo("Starting WFP transaction");
            result = WfpInterop.FwpmTransactionBegin(_engineHandle);
            if (result != 0)
            {
                Logger.LogError($"Failed to begin WFP transaction. Error code: {result}");
                return false;
            }
            Logger.LogInfo("WFP transaction started");

            if (!AddProvider())
                return false;

            if (!AddSubLayer())
                return false;

            if (!AddFilters())
                return false;

            Logger.LogInfo("Committing WFP transaction");
            result = WfpInterop.FwpmTransactionCommit(_engineHandle);
            if (result != 0)
            {
                Logger.LogError($"Failed to commit WFP transaction. Error code: {result}");
                return false;
            }
            Logger.LogInfo("WFP transaction committed");

            _isStarted = true;
            Logger.LogInfo("WFP redirect service started successfully");
            return true;
        }
        catch (Exception ex)
        {
            Logger.LogError("Error starting WFP redirect service", ex);
            Cleanup();
            return false;
        }
    }

    public bool Stop()
    {
        if (!_isStarted)
        {
            Logger.LogWarning("WFP service is not running");
            return true;
        }

        try
        {
            Logger.LogInfo("Stopping WFP redirect service");
            Cleanup();
            _isStarted = false;
            Logger.LogInfo("WFP redirect service stopped");
            return true;
        }
        catch (Exception ex)
        {
            Logger.LogError("Error stopping WFP redirect service", ex);
            return false;
        }
    }

    private bool AddProvider()
    {
        try
        {
            FWPM_PROVIDER0 provider = new();
            Guid key = _providerKey;
            provider.providerKey = key;
            provider.name = Marshal.StringToHGlobalUni("FlowReveal Provider");
            provider.description = Marshal.StringToHGlobalUni("FlowReveal HTTP Debugger Traffic Redirect");
            provider.flags = 0;

            Logger.LogInfo("Adding WFP provider");
            uint result = WfpInterop.FwpmProviderAdd(_engineHandle, ref provider, IntPtr.Zero);

            Marshal.FreeHGlobal(provider.name);
            Marshal.FreeHGlobal(provider.description);

            if (result != 0)
            {
                Logger.LogError($"Failed to add WFP provider. Error code: {result}");
                return false;
            }
            Logger.LogInfo("WFP provider added successfully");
            return true;
        }
        catch (Exception ex)
        {
            Logger.LogError("Error adding WFP provider", ex);
            return false;
        }
    }

    private bool AddSubLayer()
    {
        try
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

            Logger.LogInfo("Adding WFP sublayer");
            uint result = WfpInterop.FwpmSubLayerAdd(_engineHandle, ref subLayer, IntPtr.Zero);

            Marshal.FreeHGlobal(subLayer.name);
            Marshal.FreeHGlobal(subLayer.description);

            if (result != 0)
            {
                Logger.LogError($"Failed to add WFP sublayer. Error code: {result}");
                return false;
            }
            Logger.LogInfo("WFP sublayer added successfully");
            return true;
        }
        catch (Exception ex)
        {
            Logger.LogError("Error adding WFP sublayer", ex);
            return false;
        }
    }

    private bool AddFilters()
    {
        Logger.LogInfo("Adding WFP filters");
        bool httpResult = AddRedirectFilter(_httpPort, "HTTP Redirect");
        bool httpsResult = AddRedirectFilter(_httpsPort, "HTTPS Redirect");

        if (httpResult && httpsResult)
        {
            Logger.LogInfo("WFP filters added successfully");
        }
        else
        {
            Logger.LogError("Failed to add WFP filters");
        }

        return httpResult && httpsResult;
    }

    private bool AddRedirectFilter(int port, string name)
    {
        try
        {
            Logger.LogInfo($"Adding filter: {name} for port {port}");

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

            if (result != 0)
            {
                Logger.LogError($"Failed to add filter {name}. Error code: {result}");
                return false;
            }
            Logger.LogInfo($"Filter {name} added successfully with ID: {filterId}");
            return true;
        }
        catch (Exception ex)
        {
            Logger.LogError($"Error adding filter {name}", ex);
            return false;
        }
    }

    private void Cleanup()
    {
        Logger.LogInfo("Cleaning up WFP resources");
        if (_engineHandle != IntPtr.Zero)
        {
            foreach (ulong filterId in _filterIds)
            {
                Logger.LogInfo($"Deleting filter ID: {filterId}");
                WfpInterop.FwpmFilterDeleteById(_engineHandle, filterId);
            }
            _filterIds.Clear();

            Guid subKey = _subLayerKey;
            Logger.LogInfo("Deleting sublayer");
            WfpInterop.FwpmSubLayerDeleteByKey(_engineHandle, ref subKey);
            
            Guid provKey = _providerKey;
            Logger.LogInfo("Deleting provider");
            WfpInterop.FwpmProviderDeleteByKey(_engineHandle, ref provKey);
            
            Logger.LogInfo("Closing WFP engine");
            WfpInterop.FwpmEngineClose(_engineHandle);
            _engineHandle = IntPtr.Zero;
        }
        Logger.LogInfo("WFP resources cleaned up");
    }

    public void Dispose()
    {
        Stop();
    }

    public bool IsStarted => _isStarted;
}