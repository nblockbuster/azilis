#include <AkFileHelpers.h>
#include "tiger_io_hook.h"

extern "C"
{
    size_t ddumbe_get_wwise_file_size_by_id(uint32_t id);
    AKRESULT ddumbe_read_wwise_file_by_id(uint32_t id, void *buffer, size_t size);
}

#define FILE_HANDLE_PACKAGE_BIT (1 << 31)

AKRESULT TigerPackageIo::Init(const AkDeviceSettings &in_deviceSettings)
{
    if (in_deviceSettings.uSchedulerTypeFlags != AK_SCHEDULER_BLOCKING)
    {
        AKASSERT(!"TigerPackageIo I/O hook only works with AK_SCHEDULER_BLOCKING devices");
        return AK_Fail;
    }

    // If the Stream Manager's File Location Resolver was not set yet, set this object as the
    // File Location Resolver (this I/O hook is also able to resolve file location).
    if (!AK::StreamMgr::GetFileLocationResolver())
        AK::StreamMgr::SetFileLocationResolver(this);

    // Create a device in the Stream Manager, specifying this as the hook.
    m_deviceID = AK::StreamMgr::CreateDevice(in_deviceSettings, this);
    if (m_deviceID != AK_INVALID_DEVICE_ID)
        return AK_Success;

    return AK_Success;
}

void TigerPackageIo::Term()
{
    if (AK::StreamMgr::GetFileLocationResolver() == this)
        AK::StreamMgr::SetFileLocationResolver(NULL);
    AK::StreamMgr::DestroyDevice(m_deviceID);
}

AKRESULT TigerPackageIo::Open(
    const AkOSChar *in_pszFileName, ///< File name.
    AkOpenMode in_eOpenMode,        ///< Open mode.
    AkFileSystemFlags *in_pFlags,   ///< Special flags. Can pass NULL.
    bool &io_bSyncOpen,             ///< If true, the file must be opened synchronously. Otherwise it is left at the File Location Resolver's discretion. Return false if Open needs to be deferred.
    AkFileDesc &out_fileDesc        ///< Returned file descriptor.
)
{
    wprintf(L"Open('%s', cacheid=%08X)\n", in_pszFileName, in_pFlags->uCacheID);
    // Open the file without FILE_FLAG_OVERLAPPED and FILE_FLAG_NO_BUFFERING flags.
    AKRESULT eResult = CAkFileHelpers::OpenFile(
        in_pszFileName,
        in_eOpenMode,
        false,
        false,
        out_fileDesc.hFile);
    if (eResult == AK_Success)
    {
        ULARGE_INTEGER Temp;
        Temp.LowPart = ::GetFileSize(out_fileDesc.hFile, (LPDWORD)&Temp.HighPart);
        out_fileDesc.iFileSize = Temp.QuadPart;
        out_fileDesc.uSector = 0;
        out_fileDesc.deviceID = m_deviceID;
        out_fileDesc.pCustomParam = NULL;
        out_fileDesc.uCustomParamSize = 0;
    }
    return eResult;
}

AKRESULT TigerPackageIo::Open(
    AkFileID in_fileID,           ///< File ID.
    AkOpenMode in_eOpenMode,      ///< Open mode.
    AkFileSystemFlags *in_pFlags, ///< Special flags. Can pass NULL.
    bool &io_bSyncOpen,           ///< If true, the file must be opened synchronously. Otherwise it is left at the File Location Resolver's discretion. Return false if Open needs to be deferred.
    AkFileDesc &out_fileDesc      ///< Returned file descriptor.
)
{
    printf("Loading file ref=%08X from PM\n", in_fileID);
    size_t size = ddumbe_get_wwise_file_size_by_id(in_fileID);
    if (size == SIZE_MAX)
        return AK_FileNotFound;

    std::vector<uint8_t> buffer(size);
    if (ddumbe_read_wwise_file_by_id(in_fileID, buffer.data(), size) != AK_Success)
        return AK_Fail;

    auto fileId = this->m_nextPackageFileID++;
    this->m_packageFiles[fileId] = buffer;
    out_fileDesc.iFileSize = size;
    out_fileDesc.uSector = 0;
    out_fileDesc.deviceID = m_deviceID;
    out_fileDesc.hFile = (AkFileHandle)(fileId | FILE_HANDLE_PACKAGE_BIT);
    out_fileDesc.pCustomParam = NULL;
    out_fileDesc.uCustomParamSize = 0;

    return AK_Success;
}

AKRESULT TigerPackageIo::Read(
    AkFileDesc &in_fileDesc,             ///< File descriptor.
    const AkIoHeuristics &in_heuristics, ///< Heuristics for this data transfer.
    void *out_pBuffer,                   ///< Buffer to be filled with data.
    AkIOTransferInfo &io_transferInfo    ///< Synchronous data transfer info.
)
{
    auto uFile = uint64_t(in_fileDesc.hFile);
    if (uFile & FILE_HANDLE_PACKAGE_BIT)
    {
        auto fileId = uFile & ~FILE_HANDLE_PACKAGE_BIT;
        // printf("Read(packageFileId=%d, heuristics=%d, buffer=%p, filePos=0x%x, size=0x%x)\n", fileId, in_heuristics, out_pBuffer, io_transferInfo.uFilePosition, io_transferInfo.uRequestedSize);
        auto it = this->m_packageFiles.find(fileId);
        if (it == this->m_packageFiles.end())
            return AK_Fail;

        auto &buffer = it->second;
        if (io_transferInfo.uFilePosition + io_transferInfo.uRequestedSize > buffer.size())
            return AK_Fail;

        memcpy(out_pBuffer, buffer.data() + io_transferInfo.uFilePosition, io_transferInfo.uRequestedSize);
        return AK_Success;
    }

    printf("Read(fileDesc=%d, heuristics=%d, buffer=%p, filePos=0x%x, size=0x%x)\n", in_fileDesc.hFile, in_heuristics, out_pBuffer, io_transferInfo.uFilePosition, io_transferInfo.uRequestedSize);
    AKASSERT(out_pBuffer &&
             in_fileDesc.hFile != INVALID_HANDLE_VALUE);

    OVERLAPPED overlapped;
    overlapped.Offset = (DWORD)(io_transferInfo.uFilePosition & 0xFFFFFFFF);
    overlapped.OffsetHigh = (DWORD)((io_transferInfo.uFilePosition >> 32) & 0xFFFFFFFF);
    overlapped.hEvent = NULL;

    DWORD uSizeTransferred;

    if (::ReadFile(
            in_fileDesc.hFile,
            out_pBuffer,
            io_transferInfo.uRequestedSize,
            &uSizeTransferred,
            &overlapped))
    {
        AKASSERT(uSizeTransferred == io_transferInfo.uRequestedSize);
        return AK_Success;
    }
    return AK_Fail;
}

AKRESULT TigerPackageIo::Write(
    AkFileDesc &in_fileDesc,             ///< File descriptor.
    const AkIoHeuristics &in_heuristics, ///< Heuristics for this data transfer.
    void *in_pData,                      ///< Data to be written.
    AkIOTransferInfo &io_transferInfo    ///< Synchronous data transfer info.
)
{
    AKASSERT(in_pData &&
             in_fileDesc.hFile != INVALID_HANDLE_VALUE);

    OVERLAPPED overlapped;
    overlapped.Offset = (DWORD)(io_transferInfo.uFilePosition & 0xFFFFFFFF);
    overlapped.OffsetHigh = (DWORD)((io_transferInfo.uFilePosition >> 32) & 0xFFFFFFFF);
    overlapped.hEvent = NULL;

    DWORD uSizeTransferred;

    if (::WriteFile(
            in_fileDesc.hFile,
            in_pData,
            io_transferInfo.uRequestedSize,
            &uSizeTransferred,
            &overlapped))
    {
        AKASSERT(uSizeTransferred == io_transferInfo.uRequestedSize);
        return AK_Success;
    }
    return AK_Fail;
}

AKRESULT TigerPackageIo::Close(
    AkFileDesc &in_fileDesc ///< File descriptor.
)
{
    auto uFile = uint64_t(in_fileDesc.hFile);
    if (uFile & FILE_HANDLE_PACKAGE_BIT)
    {
        auto fileId = uFile & ~FILE_HANDLE_PACKAGE_BIT;
        printf("Close(packageFileId=%d)\n", fileId);
        this->m_packageFiles.erase(fileId);
        return AK_Success;
    }

    printf("Close(fileDesc=%d)\n", in_fileDesc.hFile);
    AKASSERT(in_fileDesc.hFile != INVALID_HANDLE_VALUE);
    return CAkFileHelpers::CloseFile(in_fileDesc.hFile);
}

AkUInt32 TigerPackageIo::GetBlockSize(
    AkFileDesc &in_fileDesc ///< File descriptor.
)
{
    return 1;
}

void TigerPackageIo::GetDeviceDesc(
    AkDeviceDesc &out_deviceDesc ///< Device description.
)
{
    static const AkOSChar szDeviceName[] = AKTEXT("TigerPackageIo");

    out_deviceDesc.bCanRead = true;
    out_deviceDesc.bCanWrite = false;
    out_deviceDesc.deviceID = m_deviceID;
    out_deviceDesc.uStringSize = AKPLATFORM::OsStrLen(szDeviceName);
    AKPLATFORM::SafeStrCpy(out_deviceDesc.szDeviceName, szDeviceName, AK_MONITOR_DEVICENAME_MAXLENGTH);
}

AkUInt32 TigerPackageIo::GetDeviceData()
{
    return 0;
}