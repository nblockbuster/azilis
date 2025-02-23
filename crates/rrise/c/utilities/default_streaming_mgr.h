/*
 * Copyright (c) 2022 Contributors to the Rrise project
 */

#ifndef DEFAULT_STREAMING_MGR_H
#define DEFAULT_STREAMING_MGR_H

#include <AK/SoundEngine/Common/AkStreamMgrModule.h>

AKRESULT InitDefaultStreamMgr(const AkDeviceSettings& deviceSettings);
AKRESULT SetBasePath(const AkOSChar* in_pszBasePath);
AKRESULT AddBasePath(const AkOSChar* in_pszBasePath);
void TermDefaultStreamMgr();

#endif // DEFAULT_STREAMING_MGR_H
