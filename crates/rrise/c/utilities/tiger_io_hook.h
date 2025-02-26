#pragma once

#include <unordered_map>
#include <vector>
#include <AK/SoundEngine/Common/AkSoundEngine.h>
#include <AK/SoundEngine/Common/AkStreamMgrModule.h>

class TigerPackageIo : public AK::StreamMgr::IAkFileLocationResolver, public AK::StreamMgr::IAkIOHookBlocking
{
public:
    TigerPackageIo() : m_deviceID(AK_INVALID_DEVICE_ID) {}

    AKRESULT Init(const AkDeviceSettings &settings);

    void Term();

    // Returns a file descriptor for a given file name (string).
    virtual AKRESULT Open(
        const AkOSChar *in_pszFileName, // File name.
        AkOpenMode in_eOpenMode,        // Open mode.
        AkFileSystemFlags *in_pFlags,   // Special flags. Can pass NULL.
        bool &io_bSyncOpen,             // If true, the file must be opened synchronously. Otherwise it is left at the File Location Resolver's discretion. Return false if Open needs to be deferred.
        AkFileDesc &out_fileDesc        // Returned file descriptor.
    );

    // Returns a file descriptor for a given file ID.
    virtual AKRESULT Open(
        AkFileID in_fileID,           // File ID.
        AkOpenMode in_eOpenMode,      // Open mode.
        AkFileSystemFlags *in_pFlags, // Special flags. Can pass NULL.
        bool &io_bSyncOpen,           // If true, the file must be opened synchronously. Otherwise it is left at the File Location Resolver's discretion. Return false if Open needs to be deferred.
        AkFileDesc &out_fileDesc      // Returned file descriptor.
    );

    virtual AKRESULT Read(
        AkFileDesc &in_fileDesc,             ///< File descriptor.
        const AkIoHeuristics &in_heuristics, ///< Heuristics for this data transfer.
        void *out_pBuffer,                   ///< Buffer to be filled with data.
        AkIOTransferInfo &in_transferInfo    ///< Synchronous data transfer info.
    );

    virtual AKRESULT Write(
        AkFileDesc &in_fileDesc,             ///< File descriptor.
        const AkIoHeuristics &in_heuristics, ///< Heuristics for this data transfer.
        void *in_pData,                      ///< Data to be written.
        AkIOTransferInfo &io_transferInfo    ///< Synchronous data transfer info.
    );

    virtual AKRESULT Close(
        AkFileDesc &in_fileDesc ///< File descriptor.
    );

    virtual AkUInt32 GetBlockSize(
        AkFileDesc &in_fileDesc ///< File descriptor.
    );

    virtual void GetDeviceDesc(
        AkDeviceDesc &out_deviceDesc ///< Device description.
    );

    virtual AkUInt32 GetDeviceData();

private:
    AkDeviceID m_deviceID;
    std::unordered_map<uint64_t, std::vector<uint8_t>> m_packageFiles;
    uint64_t m_nextPackageFileID;
};