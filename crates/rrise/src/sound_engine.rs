/*
 * Copyright (c) 2022 Contributors to the Rrise project
 */

use crate::{
    bindings::root::{AK::SoundEngine::*, *},
    settings::{AkInitSettings, AkPlatformInitSettings},
    *,
};
use ::std::convert::TryInto;
use ::std::ffi::CStr;
use ::std::fmt::Debug;

macro_rules! link_static_plugin {
    ($feature:ident) => {
        link_static_plugin![$feature, $feature]
    };
    ($feature:ident, $global_var_name:ident) => {
        paste::paste! {
            #[cfg(feature = "" $feature)]
            {
                // If max log level doesn't include debug, need to explicitly reference this variable
                // or it won't be statically linked and the plugin won't be able to be loaded.
                #[cfg(any(
                    all(
                        debug_assertions,
                        all(not(feature = "max_level_debug"), not(feature = "max_level_trace"))
                    ),
                    all(
                        not(debug_assertions),
                        all(
                            not(feature = "release_max_level_debug"),
                            not(feature = "release_max_level_trace")
                        )
                    ),
                ))]
                ::std::convert::identity(unsafe {
                    crate::bindings_static_plugins::[<$global_var_name Registration>]
                });
                log::debug!(
                    "{} has been statically loaded successfully",
                    stringify!($feature)
                )
            }
        }
    };
}

/// Initialize the sound engine.
///
/// *Warning* This function is not thread-safe.
///
/// *Remark* The initial settings should be initialized using [AkInitSettings::default]
/// and [AkPlatformInitSettings::default] to fill the structures with their
/// default settings. This is not mandatory, but it helps avoid backward compatibility problems.
///
/// *Return*
/// > - [AK_Success](AkResult::AK_Success) if the initialization was successful
/// > - [AK_MemManagerNotInitialized](AkResult::AK_MemManagerNotInitialized) if the memory manager is not available or not properly initialized
/// > - [AK_StreamMgrNotInitialized](AkResult::AK_StreamMgrNotInitialized) if the stream manager is not available or not properly initialized
/// > - [AK_SSEInstructionsNotSupported](AkResult::AK_SSEInstructionsNotSupported) if the machine does not support SSE instruction (only on the PC)
/// > - [AK_InsufficientMemory](AkResult::AK_InsufficientMemory) or [AK_Fail](AkResult::AK_Fail) if there is not enough memory available to initialize the sound engine properly
/// > - [AK_InvalidParameter](AkResult::AK_InvalidParameter) if some parameters are invalid
/// > - [AK_Fail](AkResult::AK_Fail) if the sound engine is already initialized, or if the provided settings result in insufficient resources for the initialization.
///
/// *See also*
/// > - [term]
/// > - [AkInitSettings::default]
/// > - [AkPlatformInitSettings::default]
pub fn init(
    init_settings: &mut AkInitSettings,
    platform_init_settings: &mut AkPlatformInitSettings,
) -> Result<(), AkResult> {
    let mut init_settings = init_settings.as_ak();
    let mut platform_init_settings = platform_init_settings.as_ak();
    ak_call_result![Init(&mut init_settings, &mut platform_init_settings)]?;

    link_static_plugin![AkVorbisDecoder];
    link_static_plugin![AkOggOpusDecoder]; // see Ak/Plugin/AkOpusDecoderFactory.h
    link_static_plugin![AkWemOpusDecoder]; // see Ak/Plugin/AkOpusDecoderFactory.h
    link_static_plugin![AkMeterFX];
    link_static_plugin![AkAudioInputSource];
    link_static_plugin![AkCompressorFX];
    link_static_plugin![AkDelayFX];
    link_static_plugin![AkExpanderFX];
    link_static_plugin![AkFlangerFX];
    link_static_plugin![AkGainFX];
    link_static_plugin![AkGuitarDistortionFX];
    link_static_plugin![AkHarmonizerFX];
    link_static_plugin![AkMatrixReverbFX];
    link_static_plugin![AkParametricEQFX];
    link_static_plugin![AkPeakLimiterFX];
    link_static_plugin![AkPitchShifterFX];
    link_static_plugin![AkRecorderFX];
    link_static_plugin![AkRoomVerbFX];
    link_static_plugin![AkSilenceSource];
    link_static_plugin![AkSineSource, SineSource];
    link_static_plugin![AkStereoDelayFX];
    link_static_plugin![AkSynthOneSource, AkSynthOne];
    link_static_plugin![AkTimeStretchFX];
    link_static_plugin![AkToneSource];
    link_static_plugin![AkTremoloFX];

    Ok(())
}

/// Query whether or not the sound engine has been successfully initialized.
///
/// *Warning* This function is not thread-safe. It should not be called at the same time as [init()] or [term()].
///
/// *Return* `True` if the sound engine has been initialized, `False` otherwise.
///
/// *See also*
/// > - [init]
/// > - [term]
pub fn is_initialized() -> bool {
    unsafe { IsInitialized() }
}

/// Terminates the sound engine.
///
/// If some sounds are still playing or events are still being processed when this function is
/// called, they will be stopped.
///
/// *Warning* This function is not thread-safe.
///
/// *Warning* Before calling `Term`, you must ensure that no other thread is accessing the sound engine.
///
/// *See also*
/// > - [init]
pub fn term() {
    unsafe {
        Term();
    }
}

/// Processes all commands in the sound engine's command queue.
///
/// This method has to be called periodically (usually once per game frame).
///
/// `allow_sync_render`: When AkInitSettings::b_use_lengine_thread is false, `RenderAudio` may generate
/// an audio buffer -- unless in_bAllowSyncRender is set to false. Use in_bAllowSyncRender=false
/// when calling RenderAudio from a Sound Engine callback.
///
/// *Return* Always returns [AK_Success](AkResult::AK_Success)
///
/// *See also*
/// > - [PostEvent](struct@PostEvent)
pub fn render_audio(allow_sync_render: bool) -> Result<(), AkResult> {
    ak_call_result![RenderAudio(allow_sync_render)]
}

/// Sets the offline rendering frame time in seconds.
/// When offline rendering is enabled, every call to [render_audio] will generate sample data as if this much time has elapsed.
/// If the frame time argument is less than or equal to zero, every call to [render_audio] will generate one audio buffer.
///
/// *Return* Always returns [AK_Success](AkResult::AK_Success)
pub fn set_offline_rendering_time(frame_time_in_secs: f32) -> Result<(), AkResult> {
    ak_call_result!(SetOfflineRenderingFrameTime(frame_time_in_secs))
}

/// Enables/disables offline rendering.
///  
/// *Return* Always returns [AK_Success](AkResult::AK_Success)
pub fn set_offline_rendering(enable_offline_rendering: bool) -> Result<(), AkResult> {
    ak_call_result!(SetOfflineRendering(enable_offline_rendering))
}

/// Replaces an output device previously created during engine initialization or from AddOutput, with a new output device.
/// In addition to simply removing one output device and adding a new one, the new output device will also be used on all of the master buses
/// that the old output device was associated with, and preserve all listeners that were attached to the old output device.
///
/// Like most functions of AK::SoundEngine, AddOutput is asynchronous. A successful return code merely indicates that the request is properly queued.
/// Error codes returned by this function indicate various invalid parameters. To know if this function succeeds or not, and the failure code,
/// register an AkDeviceStatusCallbackFunc callback with RegisterAudioDeviceStatusCallback.
///
/// *See also*
/// > - AK::SoundEngine::AddOutput
/// > - AK::SoundEngine::RegisterAudioDeviceStatusCallback
/// > - AK::AkDeviceStatusCallbackFunc
///
/// *Return*
/// > - [AK_InvalidID](AkResult::AK_InvalidID): The audioDeviceShareset on in_settings was not valid.
/// > - [AK_IDNotFound](AkResult::AK_IDNotFound): The audioDeviceShareset on in_settings doesn't exist.  Possibly, the Init bank isn't loaded yet or was not updated with latest changes.
/// > - [AK_DeviceNotReady](AkResult::AK_DeviceNotReady): The idDevice on in_settings doesn't match with a valid hardware device.  Either the device doesn't exist or is disabled.  Disconnected devices (headphones) are not considered "not ready" therefore won't cause this error.
/// > - [AK_DeviceNotFound](AkResult::AK_DeviceNotFound): The in_outputDeviceId provided does not match with any of the output devices that the sound engine is currently using.
/// > - [AK_InvalidParameter](AkResult::AK_InvalidParameter): Out of range parameters or unsupported parameter combinations on in_settings
/// > - [AK_Success](AkResult::AK_InvalidParameter): parameters were valid, and the remove and add will occur.
pub fn replace_output(
    settings: &mut AkOutputSettings,
    output_device_id: AkOutputDeviceID,
) -> Result<AkOutputDeviceID, AkResult> {
    let mut id = 0;
    ak_call_result!(ReplaceOutput(settings, output_device_id, &mut id) => id)
}

pub fn get_sample_rate() -> AkUInt32 {
    unsafe { GetSampleRate() }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct AkChannelConfig {
    pub num_channels: u32,
    pub config_type: u32,
    pub channel_mask: u32,
}

impl AkChannelConfig {
    pub fn set_standard(&mut self, channel_mask: u32) {
        self.config_type = 0x1;
        self.channel_mask = channel_mask;
        self.num_channels = {
            let mut chmsk = channel_mask;
            let mut ch_num: u8 = 0;
            while chmsk != 0 {
                ch_num += 1;
                chmsk &= chmsk - 1;
            }
            ch_num as u32
        }
    }

    pub fn as_ak(&self) -> bindings::root::AkChannelConfig {
        bindings::root::AkChannelConfig {
            _bitfield_1: bindings::root::AkChannelConfig::new_bitfield_1(
                self.num_channels,
                self.config_type,
                self.channel_mask,
            ),
            _bitfield_align_1: [0u32; 0],
        }
    }
}

pub fn channel_mask_to_num_channels(num_channels: u32) -> u32 {
    match num_channels {
        1 => AK_SPEAKER_SETUP_1_0_CENTER,
        2 => AK_SPEAKER_SETUP_2_0,
        3 => AK_SPEAKER_SETUP_2_1,
        4 => AK_SPEAKER_SETUP_4_0,
        5 => AK_SPEAKER_SETUP_5_0,
        6 => AK_SPEAKER_SETUP_5_1,
        7 => AK_SPEAKER_SETUP_7,
        8 => AK_SPEAKER_SETUP_7POINT1,
        _ => 0,
    }
}

/// Registers a callback used for retrieving audio samples.
/// The callback will be called from the audio thread during real-time rendering and from the main thread during offline rendering.
///
/// *See also*
/// > - [add_output]
/// > - [get_output_id]
/// > - [unregister_capture_callback]
pub fn register_capture_callback<F>(
    callback: F,
    id_output: AkOutputDeviceID,
) -> Result<(), AkResult>
where
    F: FnMut(AkAudioBuffer) + 'static,
{
    let data = Box::into_raw(Box::new(callback));
    ak_call_result!(RegisterCaptureCallback(
        Some(call_callback_as_closure::<F>),
        id_output,
        data as *mut _
    ))
}

unsafe extern "C" fn call_callback_as_closure<F>(
    cb_capture_buffer: *mut bindings::root::AkAudioBuffer,
    cb_id: u64,
    cb_cookie: *mut std::ffi::c_void,
) where
    F: FnMut(AkAudioBuffer),
{
    let callback_ptr: *mut F = cb_cookie as *mut F;
    let callback = &mut *callback_ptr;

    // Info needed: is this safe if the callback panics? Should we do something with
    // catch_unwind? Is this undefined behavior?
    callback(*cb_capture_buffer);
}

/// Unregisters a callback used for retrieving audio samples.
/// *See also*
///
/// > - [add_output]
/// > - [get_output_id]
/// > - [register_capture_callback]
pub fn unregister_capture_callback(
    callback: AkCaptureCallbackFunc,
    id_output: AkOutputDeviceID,
    cookie: *mut std::ffi::c_void,
) -> Result<(), AkResult> {
    ak_call_result!(UnregisterCaptureCallback(callback, id_output, cookie))
}

/// Gets the compounded output ID from shareset and device id.
/// Outputs are defined by their type (Audio Device shareset) and their specific system ID.  A system ID could be reused for other device types on some OS or platforms, hence the compounded ID.
/// Use 0 for in_idShareset & in_idDevice to get the Main Output ID (the one usually initialized during [init])
///
/// *Returns*
/// The id of the output
pub fn get_output_id(shareset: AkUniqueID, device: AkUInt32) -> AkOutputDeviceID {
    unsafe { GetOutputID(shareset, device) }
}

/// Sets the Output Bus Volume (direct) to be used for the specified game object.
/// The control value is a number ranging from 0.0f to 1.0f.
/// Output Bus Volumes are stored per listener association, so calling this function will override the default set of listeners. The game object in_emitterObjID will now reference its own set of listeners which will
/// be the same as the old set of listeners, but with the new associated gain.  Future changes to the default listener set will not be picked up by this game object unless ResetListenersToDefault() is called.
///
///
/// > - emitter_id: Associated emitter game object ID
/// > - listener_id: Associated listener game object ID. Pass [AK_INVALID_GAME_OBJECT] to set the Output Bus Volume for all connected listeners.
/// > - control_value: A multiplier in the range [0.0f:16.0f] ( -∞ dB to +24 dB).
///
/// *See also*
/// > - [reset_listeners_to_default]
/// > - [soundengine_environments]
/// > - [soundengine_environments_setting_dry_environment]
/// > - [soundengine_environments_id_vs_string]
///
/// *Returns*
/// > - [AK_Success](AkResult::AK_Success) when successful
/// > - [AK_InvalidFloatValue](AkResult::AK_InvalidFloatValue) if the value specified was NaN or Inf
pub fn set_game_object_output_bus_volume(
    emitter_id: AkGameObjectID,
    listener_id: AkGameObjectID,
    control_value: AkReal32,
) -> Result<(), AkResult> {
    ak_call_result!(SetGameObjectOutputBusVolume(
        emitter_id,
        listener_id,
        control_value
    ))
}

/// Unregister all game objects, or all game objects with a particular matching set of property flags.
///
/// This function to can be used to unregister all game objects.
///
/// *Return* AK_Success if successful
///
/// *Remark* Registering a game object twice does nothing. Unregistering it once unregisters it no
/// matter how many times it has been registered. Unregistering a game object while it is
/// in use is allowed, but the control over the parameters of this game object is lost.
/// For example, if a sound associated with this game object is a 3D moving sound, it will
/// stop moving once the game object is unregistered, and there will be no way to recover
/// the control over this game object.
///
/// *See also*
/// - [register_game_obj]
/// - [unregister_game_obj]
pub fn unregister_all_game_obj() -> Result<(), AkResult> {
    ak_call_result![UnregisterAllGameObj()]
}

/// Unregisters a game object.
///
/// *Return*
/// > - AK_Success if successful
/// > - AK_Fail if the specified AkGameObjectID is invalid (0 is an invalid ID)
///
/// *Remark* Registering a game object twice does nothing. Unregistering it once unregisters it no
/// matter how many times it has been registered. Unregistering a game object while it is
/// in use is allowed, but the control over the parameters of this game object is lost.
/// For example, say a sound associated with this game object is a 3D moving sound. This sound will
/// stop moving when the game object is unregistered, and there will be no way to regain control
/// over the game object.
///
/// *See also*
/// > - [register_game_obj]
/// > - [unregister_all_game_obj]
pub fn unregister_game_obj(game_object_id: AkGameObjectID) -> Result<(), AkResult> {
    ak_call_result![UnregisterGameObj(game_object_id)]
}

/// Registers a game object.
///
/// *Return*
/// > - AK_Success if successful
/// > - AK_Fail if the specified AkGameObjectID is invalid (0 is an invalid ID)
///
/// *Remark* Registering a game object twice does nothing. Unregistering it once unregisters it no
/// matter how many times it has been registered.
///
/// *See also*
/// > - [unregister_game_obj]
/// > - [unregister_all_game_obj]
pub fn register_game_obj(game_object_id: AkGameObjectID) -> Result<(), AkResult> {
    ak_call_result![RegisterGameObj(game_object_id)]
}

/// Registers a game object.
///
/// The name is just for monitoring purpose, and is not forwarded to Wwise when the `wwrelease`
/// cfg flag is on (in that case, calling this has the same effect as calling [register_game_obj()]).
///
/// *Return*
/// > - AK_Success if successful
/// > - AK_Fail if the specified AkGameObjectID is invalid (0 is an invalid ID)
///
/// *Remark* Registering a game object twice does nothing. Unregistering it once unregisters it no
/// matter how many times it has been registered.
///
/// *See also*
/// > - [register_game_obj]
/// > - [unregister_game_obj]
/// > - [unregister_all_game_obj]
pub fn register_named_game_obj<T: AsRef<str>>(
    game_object_id: AkGameObjectID,
    #[cfg_attr(wwrelease, allow(unused_variables))] name: T,
) -> Result<(), AkResult> {
    #[cfg(wwrelease)]
    return register_game_obj(game_object_id);

    #[cfg(not(wwrelease))]
    return with_cstring![name.as_ref() => cname {
        ak_call_result![RegisterGameObj1(game_object_id, cname.as_ptr())]
    }];
}

/// Sets the position of a game object.
///
/// *Warning* `position`'s orientation vectors must be normalized.
///
/// *Return*
/// > - [AK_Success](AkResult::AK_Success) when successful
/// > - [AK_InvalidParameter](AkResult::AK_InvalidParameter) if parameters are not valid.
pub fn set_position<T: Into<AkSoundPosition>>(
    game_object_id: AkGameObjectID,
    position: T,
) -> Result<(), AkResult> {
    ak_call_result![SetPosition(game_object_id, &position.into())]
}

/// Sets the default set of associated listeners for game objects that have not explicitly overridden their listener sets. Upon registration, all game objects reference the default listener set, until
/// a call to [add_listener], [remove_listener], [set_listeners] or [set_game_object_output_bus_volume] is made on that game object.
///
/// All default listeners that have previously been added via AddDefaultListener or set via SetDefaultListeners will be removed and replaced with the listeners in the array in_pListenerGameObjs.
///
/// *Return* Always returns [AK_Success](AkResult::AK_Success)
pub fn set_default_listeners(listener_ids: &[AkGameObjectID]) -> Result<(), AkResult> {
    ak_call_result![SetDefaultListeners(
        listener_ids.as_ptr(),
        listener_ids.len().try_into().unwrap()
    )]
}

/// Add a single listener to the default set of listeners. Upon registration, all game objects reference the default listener set, until
/// a call to [add_listener], [remove_listener], [set_listeners] or [set_game_object_output_bus_volume] is made on that game object.
pub fn add_default_listener(listener_id: AkGameObjectID) -> Result<(), AkResult> {
    ak_call_result![AddDefaultListener(listener_id)]
}

/// Remove a single listener from the default set of listeners. Upon registration, all game objects reference the default listener set, until
/// a call to [add_listener], [remove_listener], [set_listeners] or [set_game_object_output_bus_volume] is made on that game object.
pub fn remove_default_listener(listener_id: AkGameObjectID) -> Result<(), AkResult> {
    ak_call_result![RemoveDefaultListener(listener_id)]
}

/// Sets a game object's associated listeners.
///
/// All listeners that have previously been added via [add_listener] or set via [set_listeners] will be removed and replaced with the listeners in the array `listener_ids`.
/// Calling this function will override the default set of listeners and `emitter_id` will now reference its own, unique set of listeners.
///
/// *See also*
/// > - [set_listeners()]
/// > - [remove_listener()]
/// > - [add_default_listener()]
/// > - [remove_default_listener()]
/// > - [set_default_listeners()]
pub fn set_listeners(
    emitter_id: AkGameObjectID,
    listener_ids: &[AkGameObjectID],
) -> Result<(), AkResult> {
    ak_call_result![SetListeners(
        emitter_id,
        listener_ids.as_ptr(),
        listener_ids.len().try_into().unwrap(),
    )]
}

/// Add a single listener to a game object's set of associated listeners.
///
/// Any listeners that have previously been added or set via [add_listener] or [set_listeners] will remain as listeners and `listener_id` will be added as an additional listener.
/// Calling this function will override the default set of listeners and `emitter_id` will now reference its own, unique set of listeners.
///
/// *See also*
/// > - [set_listeners()]
/// > - [remove_listener()]
/// > - [add_default_listener()]
/// > - [remove_default_listener()]
/// > - [set_default_listeners()]
pub fn add_listener(
    emitter_id: AkGameObjectID,
    listener_id: AkGameObjectID,
) -> Result<(), AkResult> {
    ak_call_result![AddListener(emitter_id, listener_id)]
}

/// Remove a single listener from a game object's set of active listeners.
///
/// Calling this function will override the default set of listeners and `emitter_id` will now reference its own, unique set of listeners.
///
/// *See also*
/// > - [add_listener()]
/// > - [set_listeners()]
/// > - [add_default_listener()]
/// > - [remove_default_listener()]
/// > - [set_default_listeners()]
pub fn remove_listener(
    emitter_id: AkGameObjectID,
    listener_id: AkGameObjectID,
) -> Result<(), AkResult> {
    ak_call_result![RemoveListener(emitter_id, listener_id)]
}

/// Stops the current content playing associated to the specified game object ID.
///
/// If no game object is specified, all sounds will be stopped.
pub fn stop_all(game_object_id: Option<AkGameObjectID>) {
    unsafe {
        StopAll(game_object_id.unwrap_or(AK_INVALID_GAME_OBJECT));
    }
}

/// Load a bank synchronously (by Unicode string).
///
/// The bank name is passed to the Stream Manager.
///
/// A bank load request will be posted, and consumed by the Bank Manager thread.
///
/// The function returns when the request has been completely processed.
///
/// *Return*
/// The bank ID (see [get_id_from_string]). You may use this ID with [unload_bank_by_id].
/// > - [AK_Success](AkResult::AK_Success): Load or unload successful.
/// > - [AK_InsufficientMemory](AkResult::AK_InsufficientMemory): Insufficient memory to store bank data.
/// > - [AK_BankReadError](AkResult::AK_BankReadError): I/O error.
/// > - [AK_WrongBankVersion](AkResult::AK_WrongBankVersion): Invalid bank version: make sure the version of Wwise that you used to generate the SoundBanks matches that of the SDK you are currently using.
/// > - [AK_InvalidFile](AkResult::AK_InvalidFile): File specified could not be opened.
/// > - [AK_InvalidParameter](AkResult::AK_InvalidParameter): Invalid parameter, invalid memory alignment.
/// > - [AK_Fail](AkResult::AK_Fail): Load or unload failed for any other reason. (Most likely small allocation failure)
///
/// *Remarks*
/// > - The initialization bank must be loaded first.
/// > - All SoundBanks subsequently loaded must come from the same Wwise project as the
///   initialization bank. If you need to load SoundBanks from a different project, you
///   must first unload ALL banks, including the initialization bank, then load the
///   initialization bank from the other project, and finally load banks from that project.
/// > - Codecs and plug-ins must be registered before loading banks that use them.
/// > - Loading a bank referencing an unregistered plug-in or codec will result in a load bank success,
/// but the plug-ins will not be used. More specifically, playing a sound that uses an unregistered effect plug-in
/// will result in audio playback without applying the said effect. If an unregistered source plug-in is used by an event's audio objects,
/// posting the event will fail.
/// > - The sound engine internally calls get_id_from_string(name) to return the correct bank ID.
/// Therefore, in_pszString should be the real name of the SoundBank (with or without the BNK extension - it is trimmed internally),
/// not the name of the file (if you changed it), nor the full path of the file. The path should be resolved in
/// your implementation of the Stream Manager, or in the Low-Level I/O module if you use the default Stream Manager's implementation.
///
/// *See also*
/// > - [unload_bank_by_name]
/// > - [unload_bank_by_id]
/// > - [clear_banks]
/// > - [get_id_from_string]
pub fn load_bank_by_name<T: AsRef<str>>(name: T) -> Result<AkBankID, AkResult> {
    let mut bank_id = 0;
    with_cstring![name.as_ref() => cname {
        ak_call_result![LoadBank1(cname.as_ptr(), &mut bank_id) => bank_id]
    }]
}

pub fn load_bank_by_id(id: AkBankID) -> Result<(), AkResult> {
    ak_call_result![LoadBank2(id)]
}

/// Loads a bank synchronously (from in-memory data, in-place, user bank only).\n
///
/// IMPORTANT: Banks loaded from memory with in-place data MUST be unloaded using the UnloadBank function
/// providing the same memory pointer. Make sure you are using the correct UnloadBank(...) overload
///
/// Use LoadBankMemoryView when you want to manage I/O on your side. Load the bank file
/// in a buffer and pass its address to the sound engine.
/// In-memory loading is in-place: *the memory must be valid until the bank is unloaded.*
/// A bank load request will be posted, and consumed by the Bank Manager thread.
/// The function returns when the request has been completely processed.
///
/// *Return*
/// The bank ID, which is stored in the first few bytes of the bank file. You may use this
/// ID with UnloadBank().
/// > - [AK_Success](AkResult::AK_Success): Load or unload successful.
/// > - [AK_InsufficientMemory](AkResult::AK_InsufficientMemory): Insufficient memory to store bank data.
/// > - [AK_BankReadError](AkResult::AK_BankReadError): I/O error.
/// > - [AK_WrongBankVersion](AkResult::AK_WrongBankVersion): Invalid bank version: make sure the version of Wwise that you used to generate the SoundBanks matches that of the SDK you are currently using.
/// > - [AK_InvalidFile](AkResult::AK_InvalidFile): File specified could not be opened.
/// > - [AK_InvalidParameter](AkResult::AK_InvalidParameter) if some parameters are invalid, check the debug console
/// > - [AK_Fail](AkResult::AK_Fail): Load or unload failed for any other reason. (Most likely small allocation failure)
///
/// *Remarks*
/// > - The initialization bank (Init.bnk) must be loaded first.
/// > - All SoundBanks subsequently loaded must come from the same Wwise project as the
///   initialization bank. If you need to load SoundBanks from a different project, you
///   must first unload ALL banks, including the initialization bank, then load the
///   initialization bank from the other project, and finally load banks from that project.
/// > - Codecs and plug-ins must be registered before loading banks that use them.
/// > - Loading a bank referencing an unregistered plug-in or codec will result in a load bank success,
/// but the plug-ins will not be used. More specifically, playing a sound that uses an unregistered effect plug-in
/// will result in audio playback without applying the said effect. If an unregistered source plug-in is used by an event's audio objects,
/// posting the event will fail.
/// > - The memory must be aligned on platform-specific AK_BANK_PLATFORM_DATA_ALIGNMENT bytes (see AkTypes.h).
/// > - Avoid using this function for banks containing a lot of events or structure data: this data will be unpacked into the sound engine heap,
///   making the supplied bank memory redundant. For event/structure-only banks, prefer LoadBankMemoryCopy().
///
/// *See also*
/// > - [unload_bank_by_name]
/// > - [clear_banks]
pub fn load_bank_memory_view(
    data: *const ::std::os::raw::c_void,
    len: u32,
) -> Result<AkBankID, AkResult> {
    let mut bank_id = 0;
    ak_call_result![LoadBankMemoryView(data, len, &mut bank_id) => bank_id]
}

/// Loads a bank synchronously (from in-memory data, out-of-place, user bank only).\n
///
/// NOTE: Banks loaded from in-memory with out-of-place data must be unloaded using the standard UnloadBank function
/// (with no memory pointer). Make sure you are using the correct UnloadBank(...) overload
///
/// Use LoadBankMemoryCopy when you want to manage I/O on your side. Load the bank file
/// in a buffer and pass its address to the sound engine, the media section of the bank will be copied into newly
/// allocated memory.  
/// In-memory loading is out-of-place: the buffer can be release as soon as the function returns. The advantage of using this
/// over the in-place version is that there is no duplication of bank structures.
/// A bank load request will be posted, and consumed by the Bank Manager thread.
/// The function returns when the request has been completely processed.
///
/// *Return*
/// The bank ID, which is stored in the first few bytes of the bank file. You may use this
/// ID with UnloadBank().
/// > - [AK_Success](AkResult::AK_Success): Load or unload successful.
/// > - [AK_InsufficientMemory](AkResult::AK_InsufficientMemory): Insufficient memory to store bank data.
/// > - [AK_BankReadError](AkResult::AK_BankReadError): I/O error.
/// > - [AK_WrongBankVersion](AkResult::AK_WrongBankVersion): Invalid bank version: make sure the version of Wwise that you used to generate the SoundBanks matches that of the SDK you are currently using.
/// > - [AK_InvalidFile](AkResult::AK_InvalidFile): File specified could not be opened.
/// > - [AK_InvalidParameter](AkResult::AK_InvalidParameter) if some parameters are invalid, check the debug console
/// > - [AK_Fail](AkResult::AK_Fail): Load or unload failed for any other reason. (Most likely small allocation failure)
///
/// *Remarks*
/// > - The initialization bank (Init.bnk) must be loaded first.
/// > - All SoundBanks subsequently loaded must come from the same Wwise project as the
///   initialization bank. If you need to load SoundBanks from a different project, you
///   must first unload ALL banks, including the initialization bank, then load the
///   initialization bank from the other project, and finally load banks from that project.
/// > - Codecs and plug-ins must be registered before loading banks that use them.
/// > - Loading a bank referencing an unregistered plug-in or codec will result in a load bank success,
/// but the plug-ins will not be used. More specifically, playing a sound that uses an unregistered effect plug-in
/// will result in audio playback without applying the said effect. If an unregistered source plug-in is used by an event's audio objects,
/// posting the event will fail.
/// > - The memory must be aligned on platform-specific AK_BANK_PLATFORM_DATA_ALIGNMENT bytes (see AkTypes.h).
/// > - Avoid using this function for banks containing a lot of events or structure data: this data will be unpacked into the sound engine heap,
///   making the supplied bank memory redundant. For event/structure-only banks, prefer LoadBankMemoryCopy().
///
/// *See also*
/// > - [unload_bank_by_name]
/// > - [clear_banks]
pub fn load_bank_memory_copy(
    data: *const ::std::os::raw::c_void,
    len: u32,
) -> Result<AkBankID, AkResult> {
    let mut bank_id = 0;
    ak_call_result![LoadBankMemoryCopy(data, len, &mut bank_id) => bank_id]
}

/// Unload all currently loaded banks.
/// It also internally calls ClearPreparedEvents() since at least one bank must have been loaded to allow preparing events.
///
/// *Returns*
/// > - [AK_Success](AkResult::AK_Success) if successful
/// > - [AK_NotInitialized](AkResult::AK_NotInitialized) if the sound engine was not correctly initialized or if there is not enough memory to handle the command
///
/// *See also*
/// > - [unload_bank_by_name]
/// > - [load_bank_by_name]
pub fn clear_banks() -> Result<(), AkResult> {
    ak_call_result![ClearBanks()]
}

/// Unloads a bank synchronously.
/// Refer to soundengine_banks_general for a discussion on using strings and IDs.
///
/// *Return*
/// > - [AK_Success](AkResult::AK_Success) if successful or not loaded
/// > - [AK_Fail](AkResult::AK_Fail) otherwise
///
/// *Remarks*
/// > - The sound engine internally calls get_id_from_string(name) to retrieve the bank ID,
/// then it calls the synchronous version of UnloadBank() by ID.
/// Therefore, name should be the real name of the SoundBank (with or without the BNK extension - it is trimmed internally),
/// not the name of the file (if you changed it), nor the full path of the file.
/// > - In order to force the memory deallocation of the bank, sounds that use media from this bank will be stopped.
/// This means that streamed sounds or generated sounds will not be stopped.
///
/// *See also*
/// > - [load_bank_by_name]
/// > - [clear_banks]
pub fn unload_bank_by_name<T: AsRef<str>>(
    name: T,
    memory_bnk_ptr: *const ::std::os::raw::c_void,
) -> Result<(), AkResult> {
    with_cstring![name.as_ref() => cname {
        ak_call_result![UnloadBank1(cname.as_ptr(), memory_bnk_ptr)]
    }]
}

/// Unloads a bank synchronously (by ID and memory pointer).
///
/// *Return*
/// > - [AK_Success](AkResult::AK_Success) if successful or not loaded
/// > - [AK_Fail](AkResult::AK_Fail) otherwise
///
/// *Remarks*
/// > - In order to force the memory deallocation of the bank, sounds that use media from this bank will be stopped.
/// This means that streamed sounds or generated sounds will not be stopped.
///
/// *See also*
/// > - [load_bank_by_name]
/// > - [clear_banks]
pub fn unload_bank_by_id(
    id: AkBankID,
    memory_bnk_ptr: *const ::std::os::raw::c_void,
) -> Result<(), AkResult> {
    ak_call_result![UnloadBank2(id, memory_bnk_ptr)]
}

/// Universal converter from string to ID for the sound engine.
/// This function will hash the name based on a algorithm ( provided at : /AK/Tools/Common/AkFNVHash.h )
/// Note:
///		This function does return a AkUInt32, which is totally compatible with:
///		AkUniqueID, AkStateGroupID, AkStateID, AkSwitchGroupID, AkSwitchStateID, AkRtpcID, and so on...
///
/// *See also*
/// > - [PostEvent]
/// > - [set_rtpc_value]
/// > - [set_switch]
/// > - [set_state]
/// > - [post_trigger]
/// > - [set_game_object_aux_send_values]
/// > - [load_bank_by_name]
/// > - [unload_bank_by_name]
/// > - [prepare_event]
/// > - [prepare_game_syncs]
pub fn get_id_from_string<T: AsRef<str>>(string: T) -> AkUInt32 {
    unsafe { GetIDFromString1(string.as_ref().as_ptr() as *const i8) }
}

#[derive(Debug, Clone, Copy)]
pub struct AkExternalSourceInfo {
    pub external_src_cookie: AkUInt32,
    pub codec: super::AkCodecId,
    // pub file_path: *mut u16,
    // pub in_memory: *mut std::ffi::c_void, //&'a mut [u8],
    pub memory_size: AkUInt32,
    pub file_id: AkFileID,
}

impl AkExternalSourceInfo {
    // AkExternalSourceInfo (void *in_pInMemory, AkUInt32 in_uiMemorySize, AkUInt32 in_iExternalSrcCookie, AkCodecID in_idCodec)

    // TODO: This doesn't properly initialize a new in-memory only streamed media file. Still requires a file to be loaded for it to function.
    pub fn from_ptr(
        data: *mut std::ffi::c_void,
        size: AkUInt32,
        external_src_cookie: AkUInt32,
        codec: super::AkCodecId,
    ) -> Self {
        Self {
            // in_memory: data,
            memory_size: size,
            external_src_cookie,
            codec,
            // file_path: ::std::ptr::null_mut(),
            file_id: 0,
        }
    }

    // AkExternalSourceInfo (AkOSChar *in_pszFileName, AkUInt32 in_iExternalSrcCookie, AkCodecID in_idCodec)
    pub fn from_path<T: AsRef<str>>(
        file_path: T,
        external_src_cookie: AkUInt32,
        codec: super::AkCodecId,
    ) -> Self {
        let mut os_str = to_os_char(file_path);
        Self {
            // file_path: os_str.as_mut_ptr() as *mut u16,
            external_src_cookie,
            codec,
            // in_memory: ::std::ptr::null_mut(),
            memory_size: 0,
            file_id: 0,
        }
    }

    // AkExternalSourceInfo (AkFileID in_idFile, AkUInt32 in_iExternalSrcCookie, AkCodecID in_idCodec)
    pub fn from_id(id: AkFileID, external_src_cookie: AkUInt32, codec: super::AkCodecId) -> Self {
        Self {
            file_id: id,
            external_src_cookie,
            codec,
            // file_path: ::std::ptr::null_mut(),
            // in_memory: ::std::ptr::null_mut(),
            memory_size: 0,
        }
    }

    pub(crate) fn as_ak(&mut self) -> crate::bindings::root::AkExternalSourceInfo {
        crate::bindings::root::AkExternalSourceInfo {
            iExternalSrcCookie: self.external_src_cookie,
            idCodec: self.codec as u32,
            szFile: ::std::ptr::null_mut() as *mut u16,
            pInMemory: ::std::ptr::null_mut(),
            uiMemorySize: self.memory_size,
            idFile: self.file_id,
        }
    }
}

#[derive(Debug)]
/// Helper to post events to the sound engine.
///
/// Use [PostEvent::post] to post your event to the sound engine.
///
/// The callback function can be used to be noticed when markers are reached or when the event is finished.
///
/// An array of Wave file sources can be provided to resolve External Sources triggered by the event.
///
/// *Return* The playing ID of the event launched, or [AK_INVALID_PLAYING_ID] if posting the event failed
///
/// *Remarks*
/// > - If used, the array of external sources should contain the information for each external source triggered by the
/// event. When triggering an Event with multiple external sources, you need to differentiate each source
/// by using the cookie property in the External Source in the Wwise project and in AkExternalSourceInfo.
/// > - If an event triggers the playback of more than one external source, they must be named uniquely in the project
/// (therefore have a unique cookie) in order to tell them apart when filling the AkExternalSourceInfo structures.
///
/// *See also*
/// > - [render_audio]
/// > - [get_source_play_position]
pub struct PostEvent<'a> {
    game_obj_id: AkGameObjectID,
    event_id: AkID<'a>,
    flags: AkCallbackType,
    external_sources: Vec<AkExternalSourceInfo>,
    playing_id: AkPlayingID,
}

impl<'a> PostEvent<'a> {
    /// Select an event by name or by ID, to play on a given game object.
    pub fn new<T: Into<AkID<'a>>>(
        game_obj_id: AkGameObjectID,
        event_id: T,
        external_sources: Vec<AkExternalSourceInfo>,
    ) -> PostEvent<'a> {
        PostEvent {
            game_obj_id,
            event_id: event_id.into(),
            flags: AkCallbackType(0),
            external_sources: external_sources,
            playing_id: AK_INVALID_PLAYING_ID,
        }
    }

    /// Add flags before posting. Bitmask: see [AkCallbackType].
    ///
    /// *See also* [post_with_callback](Self::post_with_callback)
    pub fn add_flags(&mut self, flags: AkCallbackType) -> &mut Self {
        self.flags |= flags;
        self
    }

    /// Set flags before posting. Bitmask: see [AkCallbackType]
    ///
    /// *See also* [post_with_callback](Self::post_with_callback)
    pub fn flags(&mut self, flags: AkCallbackType) -> &mut Self {
        self.flags = flags;
        self
    }

    /// Advanced users only. Specify the playing ID to target with the event. Will Cause active
    /// actions in this event to target an existing Playing ID. Let it be [AK_INVALID_PLAYING_ID]
    /// or do not specify any for normal playback.
    pub fn playing_id(&mut self, id: AkPlayingID) -> &mut Self {
        self.playing_id = id;
        self
    }

    /// Posts the event to the sound engine.
    pub fn post(&mut self) -> Result<AkPlayingID, AkResult> {
        let mut extsrc = self
            .external_sources
            .iter_mut()
            .map(|x: &mut AkExternalSourceInfo| x.as_ak())
            .collect::<Vec<crate::bindings::root::AkExternalSourceInfo>>();

        if let AkID::Name(name) = self.event_id {
            let ak_playing_id = unsafe {
                with_cstring![name => cname {
                    PostEvent2(
                        cname.as_ptr(),
                        self.game_obj_id,
                        self.flags.0 as u32,
                        None,
                        ::std::ptr::null_mut(),
                        self.external_sources.len() as u32,
                        extsrc.as_mut_ptr(),
                        self.playing_id,
                    )
                }]
            };
            if ak_playing_id == AK_INVALID_PLAYING_ID {
                Err(AkResult::AK_Fail)
            } else {
                Ok(ak_playing_id)
            }
        } else if let AkID::ID(id) = self.event_id {
            let ak_playing_id = unsafe {
                PostEvent(
                    id,
                    self.game_obj_id,
                    self.flags.0 as u32,
                    None,
                    ::std::ptr::null_mut(),
                    self.external_sources.len() as u32,
                    extsrc.as_mut_ptr(),
                    self.playing_id,
                )
            };
            if ak_playing_id == AK_INVALID_PLAYING_ID {
                Err(AkResult::AK_Fail)
            } else {
                Ok(ak_playing_id)
            }
        } else {
            panic!("need at least an event ID or and an event name to post")
        }
    }

    /// Posts the event to the sound engine, calling `callback` according to [flags](Self::flags).
    ///
    /// `callback` can be a function or a closure.
    ///
    /// **⚡ ATTENTION ⚡**
    ///
    /// `callback` will be called on the audio thread, **not** on the current thread where you called
    /// this. This means your closure or function must access shared state in a thread-safe way.
    ///
    /// This also means the closure or function must not be long to return, or audio might sutter as
    /// it prevents the audio thread from processing buffers.
    pub fn post_with_callback<F>(&mut self, callback: F) -> Result<AkPlayingID, AkResult>
    where
        F: FnMut(crate::AkCallbackInfo) + 'static,
    {
        let mut extsrc = Vec::new(); // = self.external_sources.iter().for_each(|x| x = x.as_ak());
        for s in self.external_sources.iter_mut() {
            extsrc.push(s.as_ak());
        }
        // see http://blog.sagetheprogrammer.com/neat-rust-tricks-passing-rust-closures-to-c
        let data = Box::into_raw(Box::new(callback));

        if let AkID::Name(name) = self.event_id {
            let ak_playing_id = unsafe {
                with_cstring![name => cname {
                    PostEvent2(
                        cname.as_ptr(),
                        self.game_obj_id,
                        (self.flags | AkCallbackType::AK_EndOfEvent).0 as u32,
                        Some(Self::call_callback_as_closure::<F>),
                        data as *mut _,
                        self.external_sources.len() as u32,
                        extsrc.as_mut_ptr(),
                        self.playing_id,
                    )
                }]
            };
            if ak_playing_id == AK_INVALID_PLAYING_ID {
                Err(AkResult::AK_Fail)
            } else {
                Ok(ak_playing_id)
            }
        } else if let AkID::ID(id) = self.event_id {
            let ak_playing_id = unsafe {
                PostEvent(
                    id,
                    self.game_obj_id,
                    (self.flags | AkCallbackType::AK_EndOfEvent).0 as u32,
                    Some(Self::call_callback_as_closure::<F>),
                    data as *mut _,
                    self.external_sources.len() as u32,
                    extsrc.as_mut_ptr(),
                    self.playing_id,
                )
            };
            if ak_playing_id == AK_INVALID_PLAYING_ID {
                Err(AkResult::AK_Fail)
            } else {
                Ok(ak_playing_id)
            }
        } else {
            panic!("need at least an event ID or and an event name to post")
        }
    }

    unsafe extern "C" fn call_callback_as_closure<F>(
        cb_type: AkCallbackType,
        cb_info: *mut bindings::root::AkCallbackInfo,
    ) where
        F: FnMut(crate::AkCallbackInfo),
    {
        // see http://blog.sagetheprogrammer.com/neat-rust-tricks-passing-rust-closures-to-c

        let callback_ptr: *mut F;
        let wrapped_cb_type: crate::AkCallbackInfo;
        if cb_type.contains(AkCallbackType::AK_MusicSyncAll) {
            let cb_info = *(cb_info as *mut AkMusicSyncCallbackInfo);
            callback_ptr = cb_info._base.pCookie as *mut F;
            wrapped_cb_type = crate::AkCallbackInfo::MusicSync {
                game_obj_id: cb_info._base.gameObjID,
                playing_id: cb_info.playingID,
                segment_info: cb_info.segmentInfo,
                music_sync_type: cb_info.musicSyncType,

                user_cue_name: if cb_info.pszUserCueName.is_null() {
                    "".to_string()
                } else {
                    // Safety
                    // pszUserCueName will be valid until to_string(), which will copy the bytes from
                    // pszUserCueName onto the Rust-managed heap
                    CStr::from_ptr(cb_info.pszUserCueName as *const ::std::os::raw::c_char)
                        .to_str()
                        .unwrap()
                        .to_string()
                },
            };
        } else if cb_type.contains(AkCallbackType::AK_EndOfDynamicSequenceItem) {
            let cb_info = *(cb_info as *mut AkDynamicSequenceItemCallbackInfo);
            callback_ptr = cb_info._base.pCookie as *mut F;
            wrapped_cb_type = crate::AkCallbackInfo::DynamicSequenceItem {
                game_obj_id: cb_info._base.gameObjID,
                playing_id: cb_info.playingID,
                audio_node_id: cb_info.audioNodeID,
            };
        } else if cb_type.contains(
            AkCallbackType::AK_EndOfEvent
                | AkCallbackType::AK_MusicPlayStarted
                | AkCallbackType::AK_Starvation,
        ) {
            let cb_info = *(cb_info as *mut AkEventCallbackInfo);
            callback_ptr = cb_info._base.pCookie as *mut F;
            wrapped_cb_type = crate::AkCallbackInfo::Event {
                game_obj_id: cb_info._base.gameObjID,
                callback_type: cb_type,
                playing_id: cb_info.playingID,
                event_id: cb_info.eventID,
            };
        } else if cb_type.contains(AkCallbackType::AK_Duration) {
            let cb_info = *(cb_info as *mut AkDurationCallbackInfo);
            callback_ptr = cb_info._base._base.pCookie as *mut F;
            wrapped_cb_type = crate::AkCallbackInfo::Duration {
                game_obj_id: cb_info._base._base.gameObjID,
                playing_id: cb_info._base.playingID,
                event_id: cb_info._base.eventID,
                duration: cb_info.fDuration,
                estimated_duration: cb_info.fEstimatedDuration,
                audio_node_id: cb_info.audioNodeID,
                media_id: cb_info.mediaID,
                streaming: cb_info.bStreaming,
            };
        } else if cb_type.contains(AkCallbackType::AK_Marker) {
            let cb_info = *(cb_info as *mut AkMarkerCallbackInfo);
            callback_ptr = cb_info._base._base.pCookie as *mut F;
            wrapped_cb_type = crate::AkCallbackInfo::Marker {
                game_obj_id: cb_info._base._base.gameObjID,
                playing_id: cb_info._base.playingID,
                event_id: cb_info._base.eventID,
                identifier: cb_info.uIdentifier,
                position: cb_info.uPosition,
                label: if cb_info.strLabel.is_null() {
                    "".to_string()
                } else {
                    // Safety
                    // strLabel will be valid until to_string(), which will copy the bytes from
                    // strLabel onto the Rust-managed heap
                    CStr::from_ptr(cb_info.strLabel)
                        .to_str()
                        .unwrap()
                        .to_string()
                },
            }
        } else if cb_type.contains(AkCallbackType::AK_MIDIEvent) {
            let cb_info = *(cb_info as *mut AkMIDIEventCallbackInfo);
            callback_ptr = cb_info._base._base.pCookie as *mut F;
            wrapped_cb_type = crate::AkCallbackInfo::Midi {
                game_obj_id: cb_info._base._base.gameObjID,
                playing_id: cb_info._base.playingID,
                event_id: cb_info._base.eventID,
                midi_event: cb_info.midiEvent.into(),
            }
        } else if cb_type.contains(AkCallbackType::AK_MusicPlaylistSelect) {
            let cb_info = *(cb_info as *mut AkMusicPlaylistCallbackInfo);
            callback_ptr = cb_info._base._base.pCookie as *mut F;
            wrapped_cb_type = crate::AkCallbackInfo::MusicPlaylist {
                game_obj_id: cb_info._base._base.gameObjID,
                playing_id: cb_info._base.playingID,
                event_id: cb_info._base.eventID,
                playlist_id: cb_info.playlistID,
                num_playlist_items: cb_info.uNumPlaylistItems,
                playlist_selection: cb_info.uPlaylistSelection,
                playlist_item_done: cb_info.uPlaylistItemDone,
            }
        } else if cb_type.contains(AkCallbackType::AK_SpeakerVolumeMatrix) {
            let cb_info = *(cb_info as *mut AkSpeakerVolumeMatrixCallbackInfo);
            callback_ptr = cb_info._base._base.pCookie as *mut F;
            wrapped_cb_type = crate::AkCallbackInfo::SpeakerMatrixVolume {
                game_obj_id: cb_info._base._base.gameObjID,
                playing_id: cb_info._base.playingID,
                event_id: cb_info._base.eventID,
                input_config: cb_info.inputConfig,
                output_config: cb_info.outputConfig,
            }
        } else {
            if !cb_type.contains(AkCallbackType::AK_CallbackBits) {
                // is it safe to panic here?
                panic!("Unexpected AkCallbackType encountered: {:?}", cb_type.0);
            }

            callback_ptr = (*cb_info).pCookie as *mut F;
            wrapped_cb_type = crate::AkCallbackInfo::Default {
                game_obj_id: (*cb_info).gameObjID,
                callback_type: cb_type,
            };
        }
        let callback = &mut *callback_ptr;

        // Info needed: is this safe if the callback panics? Should we do something with
        // catch_unwind? Is this undefined behavior?
        callback(wrapped_cb_type);

        if cb_type.contains(AkCallbackType::AK_EndOfEvent) {
            // No more callbacks to process! Cleanup memory
            drop(Box::from_raw(callback_ptr));
        }
    }
}
