use std::io::SeekFrom;

use binrw::BinRead;
use modular_bitfield::prelude::*;

#[derive(BinRead, Default, Debug, Clone, PartialEq)]
pub struct AudioProperties {
    pub override_effects: u8, //bool

    _effect_count: u8,
    #[br(if(_effect_count > 0))]
    pub bypassed_effects: u8,
    #[br(count = _effect_count)]
    pub effects: Vec<AudioEffect>,

    #[br(seek_before = SeekFrom::Current(3))]
    pub output_bus_id: u32,
    pub parent_id: u32,
    pub playback_behaviour: u8,

    _param_count: u8,
    #[br(count = _param_count)]
    pub param_types: Vec<u8>,
    #[br(count = _param_count)]
    pub param_values: Vec<ParamValueType>,

    _param_pair_count: u8,
    #[br(count = _param_pair_count)]
    pub param_pair_types: Vec<u8>,
    #[br(count = _param_pair_count)]
    pub param_pairs: Vec<(f32, f32)>,

    // assume not set
    pub positioning: PositioningBehaviour,

    #[br(if(positioning.two_dimensional()))]
    _ignore: u8,

    #[br(if(positioning.update_at_each_frame() || positioning.user_defined_should_loop()))]
    pub positioning_settings: PositioningSettings,

    // assume not set
    pub aux_sends_behaviour: AuxSendsBehaviour,
    #[br(seek_before = SeekFrom::Current(4))]
    pub limit_behaviour: u8,
    pub virtual_voice_return: u8,
    pub limit_sound_instances_to: u16,
    pub virtual_voice: u8,
    pub hdr_settings: u8,

    _state_property_count: u8,
    #[br(count = _state_property_count)]
    _unk_state_property: Vec<[u8; 3]>,

    _state_group_count: u8,
    #[br(count = _state_group_count)]
    pub state_groups: Vec<AudioStateGroup>,

    _rtpc_count: u16,
    #[br(count = _rtpc_count)]
    pub rtpcs: Vec<RTPC>,
}

#[bitfield]
#[derive(BinRead, Default, Debug, Clone, PartialEq)]
#[br(map = Self::from_bytes)]
pub struct AuxSendsBehaviour {
    pub override_game_defined: bool,
    pub use_game_defined_aux_sends: bool,
    pub override_user_defined: bool,
    pub override_aux_sends: bool,
    unused: modular_bitfield::prelude::B4,
}

#[derive(Specifier, Debug, Clone, PartialEq)]
#[bits = 2]
pub enum CodeTypeMask {
    IL,
    Native,
    OPTIL,
    Runtime,
}

#[derive(Specifier, Debug, Clone, PartialEq)]
#[bits = 1]
pub enum ManagedMask {
    Managed,
    Unmanaged,
}

#[bitfield]
#[derive(BinRead, Default, Debug, Clone, PartialEq)]
#[br(map = Self::from_bytes)]
pub struct MethodImplAttributes {
    code_type: CodeTypeMask,
    managed: ManagedMask,
    forward_def: bool,
    preserve_sig: bool,
    internal_call: bool,
    synchronized: bool,
    no_inlining: bool,
    max_method_impl_val: B2,
    no_optimization: bool,
}

#[bitfield]
#[derive(BinRead, Default, Debug, Clone, PartialEq)]
#[br(map = Self::from_bytes)]
pub struct PositioningBehaviour {
    pub override_parent: bool,
    pub two_dimensional: bool,
    pub enable_2d_panner: bool,
    pub three_dimensional: bool,
    pub enable_spatialization: bool,
    pub user_defined_should_loop: bool,
    pub update_at_each_frame: bool,
    pub ignore_listener_orientation: bool,
}

#[derive(BinRead, Debug, Default, Clone, PartialEq)]
pub struct PositioningSettings {
    pub is_game_defined: u8,
    pub attenuation_id: u32,

    #[br(if(is_game_defined == 0))]
    pub game_defined: GameDefinedPositioning,
}

#[derive(BinRead, Debug, Default, Clone, PartialEq)]
pub struct GameDefinedPositioning {
    pub user_defined_play_settings: u8,
    pub transition_time: u32,

    _control_point_key_count: u32,
    #[br(count = _control_point_key_count)]
    pub control_point_keys: Vec<(f32, f32, f32, u32)>,

    _random_range_count: u32,
    #[br(count=_random_range_count)]
    pub random_range_unknowns: Vec<(u32, u32)>,
    #[br(count=_random_range_count)]
    pub random_ranges: Vec<(f32, f32, f32)>,
}

#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct AudioEffect {
    pub index: u8,
    pub id: u32,
    pub use_share_sets: u8,
    pub rendered: u8,
}

#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct ParamValueType {
    pub value: [u8; 4], // todo: type defines int, uint, default single
}

#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct AudioStateGroup {
    pub id: u32,
    pub music_change_at: u8,

    _state_with_settings_count: u8,
    #[br(count = _state_with_settings_count)]
    pub states_with_settings: Vec<AudioStateWithSettings>,
}

#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct AudioStateWithSettings {
    pub state_id: u32,
    pub settings_id: u32,
}

#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct RTPC {
    pub x: u32,
    pub is_midi: u8,
    pub is_general_settings: u8,
    pub param: u8,
    pub unk_id: u32,
    pub curve_scaling_type: u8,

    _point_count: u16,
    #[br(count = _point_count)]
    pub points: Vec<RTPCPoint>,
}

#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct RTPCPoint {
    pub x: f32,
    pub y: f32,
    pub following_shape: u8,
    _unk: [u8; 3],
}
