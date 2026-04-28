#include "FlowRevealDriver.h"

// 全局变量定义
FLOWREVEAL_GLOBALS g_FlowRevealGlobals = {0};

// Callout GUID
GUID g_FlowRevealCalloutGuid = {0x12345678, 0x1234, 0x5678, {0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF}};

// Filter GUID
GUID g_FlowRevealFilterGuid = {0x87654321, 0x4321, 0x8765, {0xEF, 0xCD, 0xAB, 0x90, 0x78, 0x56, 0x34, 0x12}};

// 驱动入口函数
NTSTATUS DriverEntry(_In_ PDRIVER_OBJECT driverObject, _In_ PUNICODE_STRING registryPath)
{
    NTSTATUS status;
    
    UNREFERENCED_PARAMETER(registryPath);
    
    DbgPrint("FlowReveal Driver v%d.%d loaded\n", DRIVER_MAJOR_VERSION, DRIVER_MINOR_VERSION);
    
    // 初始化全局变量
    RtlZeroMemory(&g_FlowRevealGlobals, sizeof(FLOWREVEAL_GLOBALS));
    
    // 设置驱动分发例程
    driverObject->DriverUnload = FlowRevealDeleteDevice;
    driverObject->MajorFunction[IRP_MJ_CREATE] = FlowRevealDispatchCreate;
    driverObject->MajorFunction[IRP_MJ_CLOSE] = FlowRevealDispatchClose;
    driverObject->MajorFunction[IRP_MJ_DEVICE_CONTROL] = FlowRevealDispatchIoctl;
    
    // 创建设备
    status = FlowRevealCreateDevice(driverObject);
    if (!NT_SUCCESS(status)) {
        DbgPrint("Failed to create device: 0x%X\n", status);
        return status;
    }
    
    // 初始化 WFP
    status = FlowRevealInitializeWfp(&g_FlowRevealGlobals);
    if (!NT_SUCCESS(status)) {
        DbgPrint("Failed to initialize WFP: 0x%X\n", status);
        FlowRevealDeleteDevice(driverObject);
        return status;
    }
    
    DbgPrint("FlowReveal Driver initialized successfully\n");
    
    return STATUS_SUCCESS;
}

// 创建设备
NTSTATUS FlowRevealCreateDevice(_In_ PDRIVER_OBJECT driverObject)
{
    NTSTATUS status;
    UNICODE_STRING deviceName, symlinkName;
    
    // 初始化设备名称
    RtlInitUnicodeString(&deviceName, FLOWREVEAL_DEVICE_NAME);
    RtlInitUnicodeString(&symlinkName, FLOWREVEAL_SYMLINK_NAME);
    
    // 创建设备对象
    status = IoCreateDevice(driverObject,
                           sizeof(FLOWREVEAL_GLOBALS),
                           &deviceName,
                           FILE_DEVICE_UNKNOWN,
                           0,
                           FALSE,
                           &g_FlowRevealGlobals.deviceObject);
    
    if (!NT_SUCCESS(status)) {
        DbgPrint("IoCreateDevice failed: 0x%X\n", status);
        return status;
    }
    
    // 设置设备扩展
    g_FlowRevealGlobals.deviceObject->DeviceExtension = &g_FlowRevealGlobals;
    
    // 创建符号链接
    status = IoCreateSymbolicLink(&symlinkName, &deviceName);
    if (!NT_SUCCESS(status)) {
        DbgPrint("IoCreateSymbolicLink failed: 0x%X\n", status);
        IoDeleteDevice(g_FlowRevealGlobals.deviceObject);
        return status;
    }
    
    // 保存名称
    g_FlowRevealGlobals.deviceName = deviceName;
    g_FlowRevealGlobals.symlinkName = symlinkName;
    
    DbgPrint("Device created successfully\n");
    
    return STATUS_SUCCESS;
}

// 删除设备
VOID FlowRevealDeleteDevice(_In_ PDRIVER_OBJECT driverObject)
{
    UNREFERENCED_PARAMETER(driverObject);
    
    DbgPrint("FlowReveal Driver unloading...\n");
    
    // 清理 WFP
    FlowRevealCleanupWfp(&g_FlowRevealGlobals);
    
    // 删除符号链接
    if (g_FlowRevealGlobals.symlinkName.Buffer != NULL) {
        IoDeleteSymbolicLink(&g_FlowRevealGlobals.symlinkName);
    }
    
    // 删除设备
    if (g_FlowRevealGlobals.deviceObject != NULL) {
        IoDeleteDevice(g_FlowRevealGlobals.deviceObject);
    }
    
    DbgPrint("FlowReveal Driver unloaded\n");
}

// 处理创建请求
NTSTATUS FlowRevealDispatchCreate(_In_ PDEVICE_OBJECT deviceObject, _In_ PIRP irp)
{
    UNREFERENCED_PARAMETER(deviceObject);
    
    irp->IoStatus.Status = STATUS_SUCCESS;
    irp->IoStatus.Information = 0;
    
    IoCompleteRequest(irp, IO_NO_INCREMENT);
    
    return STATUS_SUCCESS;
}

// 处理关闭请求
NTSTATUS FlowRevealDispatchClose(_In_ PDEVICE_OBJECT deviceObject, _In_ PIRP irp)
{
    UNREFERENCED_PARAMETER(deviceObject);
    
    irp->IoStatus.Status = STATUS_SUCCESS;
    irp->IoStatus.Information = 0;
    
    IoCompleteRequest(irp, IO_NO_INCREMENT);
    
    return STATUS_SUCCESS;
}

// 处理 IOCTL 请求
NTSTATUS FlowRevealDispatchIoctl(_In_ PDEVICE_OBJECT deviceObject, _In_ PIRP irp)
{
    NTSTATUS status = STATUS_SUCCESS;
    PIO_STACK_LOCATION ioStack = IoGetCurrentIrpStackLocation(irp);
    PFLOWREVEAL_GLOBALS globals = (PFLOWREVEAL_GLOBALS)deviceObject->DeviceExtension;
    
    switch (ioStack->Parameters.DeviceIoControl.IoControlCode) {
        case IOCTL_FLOWREVEAL_START:
            DbgPrint("IOCTL_FLOWREVEAL_START received\n");
            globals->isRunning = TRUE;
            break;
            
        case IOCTL_FLOWREVEAL_STOP:
            DbgPrint("IOCTL_FLOWREVEAL_STOP received\n");
            globals->isRunning = FALSE;
            break;
            
        case IOCTL_FLOWREVEAL_GET_PACKET:
            DbgPrint("IOCTL_FLOWREVEAL_GET_PACKET received\n");
            // 这里将实现数据包读取逻辑
            break;
            
        default:
            DbgPrint("Unknown IOCTL: 0x%X\n", ioStack->Parameters.DeviceIoControl.IoControlCode);
            status = STATUS_INVALID_DEVICE_REQUEST;
            break;
    }
    
    irp->IoStatus.Status = status;
    irp->IoStatus.Information = 0;
    
    IoCompleteRequest(irp, IO_NO_INCREMENT);
    
    return status;
}

// 初始化 WFP
NTSTATUS FlowRevealInitializeWfp(_In_ PFLOWREVEAL_GLOBALS globals)
{
    NTSTATUS status;
    
    // 打开 WFP 引擎
    status = FwpmEngineOpen0(NULL, RPC_C_AUTHN_WINNT, NULL, NULL, &globals->engineHandle);
    if (!NT_SUCCESS(status)) {
        DbgPrint("FwpmEngineOpen0 failed: 0x%X\n", status);
        return status;
    }
    
    DbgPrint("WFP Engine opened successfully\n");
    
    // 注册 Callout
    status = FlowRevealRegisterCallout(globals);
    if (!NT_SUCCESS(status)) {
        DbgPrint("FlowRevealRegisterCallout failed: 0x%X\n", status);
        FwpmEngineClose0(globals->engineHandle);
        return status;
    }
    
    return STATUS_SUCCESS;
}

// 清理 WFP
VOID FlowRevealCleanupWfp(_In_ PFLOWREVEAL_GLOBALS globals)
{
    if (globals->engineHandle != NULL) {
        // 删除过滤器
        if (globals->filterId != 0) {
            FwpmFilterDeleteById0(globals->engineHandle, globals->filterId);
        }
        
        // 删除 Callout
        if (globals->calloutId != 0) {
            FwpmCalloutDeleteById0(globals->engineHandle, globals->calloutId);
        }
        
        FwpmEngineClose0(globals->engineHandle);
        globals->engineHandle = NULL;
    }
    
    DbgPrint("WFP cleaned up\n");
}

// 注册 Callout
NTSTATUS FlowRevealRegisterCallout(_In_ PFLOWREVEAL_GLOBALS globals)
{
    NTSTATUS status;
    FWPS_CALLOUT callout = {0};
    FWPM_CALLOUT0 wfpCallout = {0};
    FWPM_FILTER0 filter = {0};
    
    // 初始化 Callout
    callout.calloutKey = g_FlowRevealCalloutGuid;
    callout.classifyFn = FlowRevealClassifyFn;
    callout.notifyFn = FlowRevealNotifyFn;
    callout.flowDeleteFn = NULL;
    
    // 注册 Callout
    status = FwpsCalloutRegister0(globals->engineHandle, &callout, &globals->calloutId);
    if (!NT_SUCCESS(status)) {
        DbgPrint("FwpsCalloutRegister0 failed: 0x%X\n", status);
        return status;
    }
    
    DbgPrint("Callout registered: %I64u\n", globals->calloutId);
    
    // 初始化 WFP Callout 结构
    wfpCallout.calloutKey = g_FlowRevealCalloutGuid;
    wfpCallout.displayData.name = L"FlowReveal Callout";
    wfpCallout.displayData.description = L"FlowReveal Network Capture Callout";
    wfpCallout.applicableLayer = FWPS_LAYER_STREAM_V4;
    
    // 添加 Callout 到 WFP
    status = FwpmCalloutAdd0(globals->engineHandle, &wfpCallout, NULL, NULL);
    if (!NT_SUCCESS(status)) {
        DbgPrint("FwpmCalloutAdd0 failed: 0x%X\n", status);
        FwpsCalloutUnregisterById0(globals->calloutId);
        return status;
    }
    
    // 初始化过滤器
    filter.filterKey = g_FlowRevealFilterGuid;
    filter.calloutKey = g_FlowRevealCalloutGuid;
    filter.displayData.name = L"FlowReveal Filter";
    filter.displayData.description = L"FlowReveal Network Capture Filter";
    filter.layerKey = FWPM_LAYER_STREAM_V4;
    filter.subLayerKey = FWPM_SUBLAYER_INSPECTION;
    filter.weight.type = FWP_UINT8;
    filter.weight.uint8 = 0x10;
    filter.action.type = FWP_ACTION_CALLOUT_TERMINATING;
    filter.action.calloutKey = g_FlowRevealCalloutGuid;
    
    // 添加过滤器
    status = FwpmFilterAdd0(globals->engineHandle, &filter, NULL, &globals->filterId);
    if (!NT_SUCCESS(status)) {
        DbgPrint("FwpmFilterAdd0 failed: 0x%X\n", status);
        FwpmCalloutDeleteById0(globals->engineHandle, globals->calloutId);
        FwpsCalloutUnregisterById0(globals->calloutId);
        return status;
    }
    
    DbgPrint("Filter added: %I64u\n", globals->filterId);
    
    return STATUS_SUCCESS;
}

// 分类函数 - 处理数据包
VOID FlowRevealClassifyFn(_In_ const FWPS_INCOMING_VALUES* inFixedValues,
                          _In_ const FWPS_INCOMING_METADATA_VALUES* inMetaValues,
                          _Inout_opt_ PVOID layerData,
                          _In_opt_ const void* classifyContext,
                          _In_ const FWPS_FILTER* filter,
                          _In_ UINT64 flowContext,
                          _Inout_ FWPS_CLASSIFY_OUT* classifyOut)
{
    UNREFERENCED_PARAMETER(inFixedValues);
    UNREFERENCED_PARAMETER(classifyContext);
    UNREFERENCED_PARAMETER(filter);
    UNREFERENCED_PARAMETER(flowContext);
    
    PFLOWREVEAL_GLOBALS globals = &g_FlowRevealGlobals;
    
    // 如果未运行，直接允许流量
    if (!globals->isRunning) {
        classifyOut->actionType = FWP_ACTION_PERMIT;
        return;
    }
    
    // 获取数据包信息
    if (inMetaValues != NULL && layerData != NULL) {
        // 这里将实现数据包捕获逻辑
        // 获取进程 ID
        UINT32 processId = (inMetaValues->processId != NULL) ? *inMetaValues->processId : 0;
        
        DbgPrint("Packet captured from PID: %u\n", processId);
    }
    
    // 允许流量继续
    classifyOut->actionType = FWP_ACTION_PERMIT;
}

// 通知函数
NTSTATUS FlowRevealNotifyFn(_In_ FWPS_CALLOUT_NOTIFY_TYPE notifyType,
                           _In_ const FWPS_CALLOUT* callout)
{
    UNREFERENCED_PARAMETER(callout);
    
    switch (notifyType) {
        case FWPS_CALLOUT_NOTIFY_ADD:
            DbgPrint("Callout added\n");
            break;
            
        case FWPS_CALLOUT_NOTIFY_DELETE:
            DbgPrint("Callout deleted\n");
            break;
            
        case FWPS_CALLOUT_NOTIFY_ENABLE:
            DbgPrint("Callout enabled\n");
            break;
            
        case FWPS_CALLOUT_NOTIFY_DISABLE:
            DbgPrint("Callout disabled\n");
            break;
    }
    
    return STATUS_SUCCESS;
}