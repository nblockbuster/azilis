use binrw::BinRead;

#[derive(BinRead, Default, Debug, Clone, PartialEq)]
pub struct Event {
    pub id: u32,
    _action_count: u8,
    #[br(count = _action_count)]
    pub action_ids: Vec<u32>,
}

#[derive(BinRead, Default, Debug, Clone, PartialEq)]
pub struct EventAction {
    pub id: u32,
    pub scope: u8,
    pub action_type: EventActionType,
    pub object_id: u32,
    pub object_id_2: u8,

    _param_count: u8,
    #[br(count = _param_count)]
    pub param_types: Vec<u8>,
    #[br(count = _param_count)]
    pub param_values: Vec<u32>,

    _param_pair_count: u8,
    #[br(count = _param_pair_count)]
    pub param_pair_types: Vec<u8>,
    #[br(count = _param_pair_count)]
    pub param_pair_values: Vec<(f32, f32)>,

    #[br(if(action_type.has_settings()), args { kind: action_type.clone() } )]
    pub settings: EventActionSettings,
}

#[derive(BinRead, Default, Debug, Clone, PartialEq)]
#[br(import { kind: EventActionType } )]
pub enum EventActionSettings {
    #[default]
    None,
    #[br(pre_assert(kind == EventActionType::Stop))]
    Stop((u8, u8, EventActionExceptions)),
    #[br(pre_assert(kind == EventActionType::Pause))]
    Pause((u8, u8, EventActionExceptions)),
    #[br(pre_assert(kind == EventActionType::Resume))]
    Resume((u8, u8, EventActionExceptions)),
    #[br(pre_assert(kind == EventActionType::Play))]
    Play((u8, u32)),
    #[br(pre_assert(kind == EventActionType::Seek))]
    Seek((u8, f32, u32, u32, u8)),
}

#[derive(BinRead, Default, Debug, Clone, PartialEq)]
pub struct EventActionExceptions {
    _count: u8,
    #[br(count = _count)]
    pub exceptions: Vec<(u32, u8)>,
}

#[derive(BinRead, Default, Debug, Clone, PartialEq)]
#[br(repr(u8))]
pub enum EventActionType {
    #[default]
    None,
    Stop,
    Pause,
    Resume,
    Play,
    Trigger,
    Mute,
    UnMute,
    SetVoicePitch,
    ResetVoicePitch,
    SetVoiceVolume,
    ResetVoiceVolume,
    SetBusVolume,
    ResetBusVolume,
    SetVoiceLowPassFilter,
    ResetVoiceLowPassFilter,
    EnableState,
    DisableState,
    SetState,
    SetGameParameter,
    ResetGameParameter,
    SetSwitch = 0x19,
    ToggleBypass,
    ResetBypassEffect,
    Break,
    Unk1d,
    Seek,
    Unk1f,
    Unk20,
    Unk21,
    Unk22,
}

impl EventActionType {
    pub fn has_settings(&self) -> bool {
        matches!(
            &self,
            Self::Stop | Self::Pause | Self::Resume | Self::Play | Self::Seek
        )
    }
}
