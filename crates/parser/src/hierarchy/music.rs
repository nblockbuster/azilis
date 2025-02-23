use super::audio::*;

use binrw::BinRead;
use std::{
    fmt::Debug,
    hash::Hash,
    io::{Cursor, SeekFrom},
};

#[derive(BinRead, Default, Debug, Clone, PartialEq)]
pub struct MusicTrack {
    pub id: u32,
    pub midi_behaviour: u8,

    _sound_count: u32,
    #[br(count = _sound_count)]
    pub sounds: Vec<Sound>,

    _time_param_count: u32,
    #[br(count = _time_param_count)]
    pub time_parameters: Vec<MusicTrackTimeParameter>,
    #[br(if(_time_param_count > 0))]
    pub sub_track_count: u32,

    _curve_count: u32,
    #[br(count = _curve_count)]
    pub curves: Vec<MusicTrackCurve>,

    pub properties: AudioProperties,
    pub track_type: MusicTrackType,

    #[br(if(track_type == MusicTrackType::Switch))]
    pub switch_params: MusicSwitchParams,

    pub look_ahead_time: u32,
}

#[derive(BinRead, Default, Debug, Clone, PartialEq, Eq, Hash)]
#[br(repr(u8))]
pub enum MusicTrackType {
    #[default]
    Normal,
    RandomStep,
    SequenceStep,
    Switch,
}

#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct Sound {
    pub unk4: u8,
    pub unk5: u8,
    pub conversion: u8, //SoundConversion
    pub unk7: u8,
    pub source: u8, // SoundSource
    pub audio_id: u32,
    pub audio_len: u32,
    pub audio_type: u8, // SoundType
}

#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct MusicTrackTimeParameter {
    pub sub_track_index: u32,
    pub audio_id: u32,
    pub event_id: u32,
    pub begin_offset: f64,
    pub begin_trim_offset: f64,
    pub end_trim_offset: f64,
    pub end_offset: f64,
}

#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct MusicTrackCurve {
    pub time_parameter_index: u32,
    pub curve_type: u32, // MusicFadeCurveType
    pub point_count: u32,

    #[br(count = point_count)]
    pub points: Vec<MusicCurvePoint>,
}

#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct MusicCurvePoint {
    pub x: f32,
    pub y: f32,
    pub following_shape: u32, // AudioCurveShapeUInt
}

#[derive(BinRead, Debug, Clone, Default, PartialEq)]
pub struct MusicSwitchParams {
    _unk0: u8,
    pub group_id: u32,
    pub default_switch_state_id: u32,

    pub sub_track_count: u32,
    #[br(count = sub_track_count)]
    pub associated_switch_state_ids: Vec<u32>,

    pub fade_out_dur: u32,
    pub fade_out_shape: u32,
    pub face_out_offset: u32,
    pub exit_souce_at: u32,
    pub exit_source_at_cueid: u32,

    pub fade_in_dur: u32,
    pub fade_in_shape: u32,
    pub fade_in_offset: u32,
}

#[derive(BinRead, Default, Debug, Clone, PartialEq)]
pub struct MusicSwitchContainer {
    pub id: u32,
    pub midi_behaviour: u8,
    pub properties: AudioProperties,

    _child_count: u32,
    #[br(count = _child_count)]
    pub child_ids: Vec<u32>,

    pub grid_period_time: f64,
    pub grid_offset_time: f64,
    pub tempo: f32,
    pub time_signature: (u8, u8),

    #[br(seek_before=SeekFrom::Current(1))]
    _stinger_count: u32,
    #[br(count = _stinger_count)]
    pub stingers: Vec<MusicStinger>,

    _transition_count: u32,
    #[br(count = _transition_count)]
    pub transitions: Vec<MusicTransition>,

    pub continue_on_group_change: u8,
    _group_count: u32,
    #[br(count = _group_count)]
    pub group_ids: Vec<u32>,
    #[br(count = _group_count)]
    pub group_types: Vec<u8>,

    pub path_selection_length: u32,
    pub use_weighted: u8,
    // #[br(args(path_selection_length))]
    #[br(ignore)]
    pub paths: AudioPathNode,
    #[br(count = path_selection_length)]
    pub path_sections: Vec<u8>,
}

impl MusicSwitchContainer {
    pub fn read_path_element(
        &mut self,
        children_start_at: u32,
    ) -> anyhow::Result<AudioPathElement> {
        if self.path_selection_length % 0xC > 0 {
            return Err(anyhow::anyhow!(
                "path_selection_length should be exactly multiple of 0xC"
            ));
        }
        if children_start_at as usize * 0xC > self.path_sections.len() {
            return Err(anyhow::anyhow!("Index exceeded path size"));
        }
        let section = &self.path_sections[children_start_at as usize * 0xC..];
        let mut cur = Cursor::new(section);

        let from_id = u32::read_le(&mut cur)?;
        let audio_id = u32::read_le(&mut cur)?;

        let children_start_at_index = audio_id as u16;
        let child_count = (audio_id >> 16) as u16;

        let weight = u16::read_le(&mut cur)?;
        let probability = u16::read_le(&mut cur)?;

        let is_endpoint = if children_start_at_index < self.path_sections.len() as u16
            && children_start_at_index > children_start_at as u16
            && child_count < self.path_sections.len() as u16 - children_start_at as u16
        {
            false
        } else {
            true
        };

        Ok(if is_endpoint {
            AudioPathElement::MusicEndpoint(MusicPathEndpoint {
                from_id,
                audio_id,
                weight,
                probability,
            })
        } else {
            let mut children = Vec::new();
            for i in 0..child_count as u32 {
                let elem = self.read_path_element(children_start_at_index as u32 + i)?;
                children.push(elem);
            }
            AudioPathElement::AudioPath(AudioPathNode {
                from_id,
                weight,
                probability,
                children_start_at_index,
                children,
            })
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AudioPathNode {
    pub from_id: u32,
    pub weight: u16,
    pub probability: u16,

    pub children_start_at_index: u16,
    pub children: Vec<AudioPathElement>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MusicPathEndpoint {
    pub from_id: u32,
    pub weight: u16,
    pub probability: u16,

    pub audio_id: u32,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum AudioPathElement {
    #[default]
    None,
    AudioPath(AudioPathNode),
    MusicEndpoint(MusicPathEndpoint),
}

#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct MusicStinger {
    pub trigger_id: u32,
    pub segment_id: u32,
    pub play_at: u32,
    pub cue_id: u32,
    pub do_not_repeat_in: u32,
    pub allow_playing_in_next_segment: u32,
}

#[derive(BinRead, Debug, Clone, PartialEq)]
pub struct MusicTransition {
    _src_id_count: u32,
    #[br(count = _src_id_count)]
    pub source_ids: Vec<u32>,
    _dst_id_count: u32,
    #[br(count = _dst_id_count)]
    pub destination_ids: Vec<u32>,
    pub fade_out_duration: u32,
    pub fade_out_curve: u32,
    pub fade_out_offset: u32,
    pub exit_source_at: u32,
    pub exit_source_at_id: u32,
    pub play_post_exit: u8,
    pub fade_in_duration: u32,
    pub fade_in_curve: u32,
    pub fade_in_offset: u32,
    pub custom_cue_filter_id: u32,
    pub jump_to_playlist_item_id: u32,
    pub destination_sync_to: u32,
    pub play_pre_entry: u8,
    pub match_source_cue_name: u8,
    pub use_transition_segment: u8,
    #[br(if(use_transition_segment == 1))]
    pub transition_segment: TransitionSegment,
}

#[derive(BinRead, Debug, Clone, Default, PartialEq)]
pub struct TransitionSegment {
    pub id: u32,
    pub fade_in_duration: u32,
    pub fade_in_curve: u32,
    pub fade_in_offset: u32,
    pub fade_out_duration: u32,
    pub fade_out_curve: u32,
    pub fade_out_offset: u32,
    pub play_pre_entry: u8,
    pub play_post_exit: u8,
}
