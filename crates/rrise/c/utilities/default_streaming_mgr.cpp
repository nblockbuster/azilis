/*
 * Copyright (c) 2022 Contributors to the Rrise project
 */

#include "default_streaming_mgr.h"
#include <AkFilePackageLowLevelIOBlocking.h>

static CAkFilePackageLowLevelIOBlocking g_lowLevelIO;

AKRESULT InitDefaultStreamMgr(const AkDeviceSettings& deviceSettings)
{
    // AKRESULT r = g_lowLevelIO.Init(deviceSettings);
    // if (r == AK_Success) {
    //     g_lowLevelIO.SetBasePath(basePath);
    // }

	return g_lowLevelIO.Init(deviceSettings);
}

AKRESULT SetBasePath(const AkOSChar* in_pszBasePath)
{
	return g_lowLevelIO.SetBasePath( in_pszBasePath );
}

AKRESULT AddBasePath(const AkOSChar* in_pszBasePath)
{
	return g_lowLevelIO.AddBasePath( in_pszBasePath );
}

void TermDefaultStreamMgr()
{
	g_lowLevelIO.Term();
	if (AK::IAkStreamMgr::Get())
	{
		AK::IAkStreamMgr::Get()->Destroy();
	}
}
