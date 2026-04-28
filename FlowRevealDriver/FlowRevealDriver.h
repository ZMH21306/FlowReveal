#pragma once

#include <ntddk.h>
#include <fwpsk.h>
#include <fwpmk.h>

#pragma warning(push)
#pragma warning(disable: 4201)
#include <pshpack4.h>
#pragma warning(pop)

// 驱动版本
#define DRIVER_MAJOR_VERSION    1
#define DRIVER_MINOR_VERSION    0

// 设备名称
#define FLOWREVEAL_DEVICE_NAME     L"\\Device\\FlowReveal"
#define FLOWREVEAL_SYMLINK_NAME    L"\\DosDevices\\FlowReveal"

// IOCTL 控制码
#define FLOWREVEAL_IOCTL_BASE        0x800
#define IOCTL_FLOWREVEAL_GET_PACKET  CTL_CODE(FILE_DEVICE_UNKNOWN, FLOWREVEAL_IOCTL_BASE + 0, METHOD_BUFFERED, FILE_READ_DATA)
#define IOCTL_FLOWREVEAL_START       CTL_CODE(FILE_DEVICE_UNKNOWN, FLOWREVEAL_IOCTL_BASE + 1, METHOD_BUFFERED, FILE_WRITE_DATA)
#define IOCTL_FLOWREVEAL_STOP        CTL_CODE(FILE_DEVICE_UNKNOWN, FLOWREVEAL_IOCTL_BASE + 2, METHOD_BUFFERED, FILE_WRITE_DATA)

// 包数据结构
typedef struct _FLOWREVEAL_PACKET_INFO {
    UINT64 timestamp;
    UINT32 processId;
    UINT32 remotePort;
    UINT32 localPort;
    UINT8  protocol;
    UINT8  ipVersion;
    UINT8  reserved[2];
    UINT32 dataLength;
    UINT8  data[0];
} FLOWREVEAL_PACKET_INFO, *PFLOWREVEAL_PACKET_INFO;

// 全局数据结构
typedef struct _FLOWREVEAL_GLOBALS {
    PDEVICE_OBJECT deviceObject;
    PDEVICE_OBJECT nextDeviceObject;
    UNICODE_STRING deviceName;
    UNICODE_STRING symlinkName;
    HANDLE engineHandle;
    UINT64 calloutId;
    UINT64 filterId;
    BOOLEAN isRunning;
    KSPIN_LOCK packetQueueLock;
    LIST_ENTRY packetQueue;
    KEVENT packetEvent;
    PETHREAD workerThread;
} FLOWREVEAL_GLOBALS, *PFLOWREVEAL_GLOBALS;

// 全局变量声明
extern FLOWREVEAL_GLOBALS g_FlowRevealGlobals;

// 函数声明
NTSTATUS FlowRevealCreateDevice(_In_ PDRIVER_OBJECT driverObject);
VOID FlowRevealDeleteDevice(_In_ PDRIVER_OBJECT driverObject);
NTSTATUS FlowRevealDispatchCreate(_In_ PDEVICE_OBJECT deviceObject, _In_ PIRP irp);
NTSTATUS FlowRevealDispatchClose(_In_ PDEVICE_OBJECT deviceObject, _In_ PIRP irp);
NTSTATUS FlowRevealDispatchIoctl(_In_ PDEVICE_OBJECT deviceObject, _In_ PIRP irp);
NTSTATUS FlowRevealInitializeWfp(_In_ PFLOWREVEAL_GLOBALS globals);
VOID FlowRevealCleanupWfp(_In_ PFLOWREVEAL_GLOBALS globals);
NTSTATUS FlowRevealRegisterCallout(_In_ PFLOWREVEAL_GLOBALS globals);
VOID FlowRevealClassifyFn(_In_ const FWPS_INCOMING_VALUES* inFixedValues,
                          _In_ const FWPS_INCOMING_METADATA_VALUES* inMetaValues,
                          _Inout_opt_ PVOID layerData,
                          _In_opt_ const void* classifyContext,
                          _In_ const FWPS_FILTER* filter,
                          _In_ UINT64 flowContext,
                          _Inout_ FWPS_CLASSIFY_OUT* classifyOut);
NTSTATUS FlowRevealNotifyFn(_In_ FWPS_CALLOUT_NOTIFY_TYPE notifyType,
                           _In_ const FWPS_CALLOUT* callout);
PDEVICE_OBJECT FlowRevealAttachDevice(_In_ PDEVICE_OBJECT deviceObject);
VOID FlowRevealDetachDevice(_In_ PDEVICE_OBJECT deviceObject);
