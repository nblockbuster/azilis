/*
 * Copyright (c) 2022 Contributors to the Rrise project
 */

#ifndef TIGER_STREAMING_MGR_H
#define TIGER_STREAMING_MGR_H

#include <AK/SoundEngine/Common/AkStreamMgrModule.h>

AKRESULT InitTigerStreamMgr(const AkDeviceSettings& deviceSettings);
void TermTigerStreamMgr();

#endif // DEFAULT_STREAMING_MGR_H
