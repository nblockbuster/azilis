pub mod audio;
pub mod event;
pub mod music;

use std::io::{Cursor, Seek, SeekFrom};

use event::*;
use music::*;

use binrw::{BinRead, BinReaderExt};
use rayon::iter::{IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator};

#[derive(BinRead, Debug, Default, Clone, PartialEq)]
#[br(import { h: HierarchyObjectHeader })]
pub enum HierarchyObjectType {
    #[default]
    #[br(pre_assert(h.ty == 0))]
    Unknown,
    #[br(pre_assert(h.ty == 1))]
    Settings,
    #[br(pre_assert(h.ty == 2))]
    Sound,

    #[br(pre_assert(h.ty == 3))]
    EventAction(EventAction),
    #[br(pre_assert(h.ty == 4))]
    Event(Event),

    #[br(pre_assert(h.ty == 5))]
    Container,
    #[br(pre_assert(h.ty == 6))]
    SwitchContainer,
    #[br(pre_assert(h.ty == 7))]
    ActorMixer,
    #[br(pre_assert(h.ty == 8))]
    AudioBus,
    #[br(pre_assert(h.ty == 9))]
    BlendContainer,
    #[br(pre_assert(h.ty == 10))]
    MusicSegment,

    #[br(pre_assert(h.ty == 11))]
    MusicTrack(MusicTrack),

    #[br(pre_assert(h.ty == 12))]
    MusicSwitchContainer(MusicSwitchContainer),
    #[br(pre_assert(h.ty == 13))]
    MusicPlaylistContainer,
    #[br(pre_assert(h.ty == 14))]
    Attenuation,
    #[br(pre_assert(h.ty == 15))]
    DialogueEvent,
    #[br(pre_assert(h.ty == 16))]
    MotionBus,
    #[br(pre_assert(h.ty == 17))]
    MotionEffect,
    #[br(pre_assert(h.ty == 18))]
    Effect,
    #[br(pre_assert(h.ty == 19))]
    EffectCustom,
    #[br(pre_assert(h.ty == 20))]
    AuxiliaryBus,
    #[br(pre_assert(h.ty == 22))]
    Unk22,
}

impl From<u8> for HierarchyObjectType {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::Settings,
            2 => Self::Sound,
            3 => Self::EventAction(EventAction::default()),
            4 => Self::Event(Event::default()),
            5 => Self::Container,
            6 => Self::SwitchContainer,
            7 => Self::ActorMixer,
            8 => Self::AudioBus,
            9 => Self::BlendContainer,
            10 => Self::MusicSegment,
            11 => Self::MusicTrack(MusicTrack::default()),
            12 => Self::MusicSwitchContainer(MusicSwitchContainer::default()),
            13 => Self::MusicPlaylistContainer,
            14 => Self::Attenuation,
            15 => Self::DialogueEvent,
            16 => Self::MotionBus,
            17 => Self::MotionEffect,
            18 => Self::Effect,
            19 => Self::EffectCustom,
            20 => Self::AuxiliaryBus,
            _ => Self::Unknown,
        }
    }
}

pub trait ExtractInner<T> {
    fn extract_inner(&self) -> Option<&T>;
    fn extract_inner_mut(&mut self) -> Option<&mut T>;
    fn extract_inner_cloned(&self) -> Option<T>;
}

impl ExtractInner<EventAction> for HierarchyObjectType {
    fn extract_inner(&self) -> Option<&EventAction> {
        if let HierarchyObjectType::EventAction(inner) = self {
            Some(inner)
        } else {
            None
        }
    }
    fn extract_inner_mut(&mut self) -> Option<&mut EventAction> {
        if let HierarchyObjectType::EventAction(inner) = self {
            Some(inner)
        } else {
            None
        }
    }
    fn extract_inner_cloned(&self) -> Option<EventAction> {
        if let HierarchyObjectType::EventAction(inner) = self {
            Some(inner.clone())
        } else {
            None
        }
    }
}

impl ExtractInner<Event> for HierarchyObjectType {
    fn extract_inner(&self) -> Option<&Event> {
        if let HierarchyObjectType::Event(inner) = self {
            Some(inner)
        } else {
            None
        }
    }
    fn extract_inner_mut(&mut self) -> Option<&mut Event> {
        if let HierarchyObjectType::Event(inner) = self {
            Some(inner)
        } else {
            None
        }
    }
    fn extract_inner_cloned(&self) -> Option<Event> {
        if let HierarchyObjectType::Event(inner) = self {
            Some(inner.clone())
        } else {
            None
        }
    }
}

impl ExtractInner<MusicTrack> for HierarchyObjectType {
    fn extract_inner(&self) -> Option<&MusicTrack> {
        if let HierarchyObjectType::MusicTrack(inner) = self {
            Some(inner)
        } else {
            None
        }
    }
    fn extract_inner_mut(&mut self) -> Option<&mut MusicTrack> {
        if let HierarchyObjectType::MusicTrack(inner) = self {
            Some(inner)
        } else {
            None
        }
    }
    fn extract_inner_cloned(&self) -> Option<MusicTrack> {
        if let HierarchyObjectType::MusicTrack(inner) = self {
            Some(inner.clone())
        } else {
            None
        }
    }
}

impl ExtractInner<MusicSwitchContainer> for HierarchyObjectType {
    fn extract_inner(&self) -> Option<&MusicSwitchContainer> {
        if let HierarchyObjectType::MusicSwitchContainer(inner) = self {
            Some(inner)
        } else {
            None
        }
    }
    fn extract_inner_mut(&mut self) -> Option<&mut MusicSwitchContainer> {
        if let HierarchyObjectType::MusicSwitchContainer(inner) = self {
            Some(inner)
        } else {
            None
        }
    }
    fn extract_inner_cloned(&self) -> Option<MusicSwitchContainer> {
        if let HierarchyObjectType::MusicSwitchContainer(inner) = self {
            Some(inner.clone())
        } else {
            None
        }
    }
}

#[derive(BinRead, Debug, Clone)]
pub struct HierarchyChunk {
    pub object_count: u32,
    #[br(ignore)]
    pub objects: Vec<HierarchyObject>,
}

impl HierarchyChunk {
    // #[profiling::function]
    pub fn read_hierarchy(
        &mut self,
        cur: &mut Cursor<&[u8]>,
    ) -> anyhow::Result<Vec<HierarchyObject>> {
        #[cfg(feature = "profiler")]
        profiling::scope!("HierarchyChunk::read_hierarchy");
        for _ in 0..self.object_count {
            let t: u8 = cur.read_le()?;
            cur.seek_relative(-1)?;
            let pos = cur.position();
            let mut a = pos + 5;
            if t > 32 {
                a -= 5;
                cur.seek(SeekFrom::Start(a))?;
            }
            let mut object: HierarchyObject = cur.read_le()?;
            let end = a + object.header.len as u64;
            #[cfg(feature = "profiler")]
            profiling::scope!(
                "HierarchyChunk::read_hierarchy.inner",
                format!("pos {}, len {}, end {}", pos, object.header.len, end).as_str()
            );
            let obj =
                HierarchyObjectType::read_le_args(cur, binrw::args! {h: object.header.clone()})?;
            object.obj = obj;
            cur.seek(SeekFrom::Start(end))?;
            self.objects.push(object);
        }
        Ok(self.objects.clone())
    }

    // #[profiling::function]
    pub fn get_all_by_type<T>(&self) -> Vec<&T>
    where
        HierarchyObjectType: ExtractInner<T>,
        T: Clone + Send + Sync,
    {
        #[cfg(feature = "profiler")]
        profiling::scope!("HierarchyChunk::get_all_by_type");
        self.objects
            .par_iter()
            .filter_map(|x| x.obj.extract_inner())
            .collect::<Vec<&T>>()
    }

    // #[profiling::function]
    pub fn get_all_by_type_mut<T>(&mut self) -> Vec<&mut T>
    where
        HierarchyObjectType: ExtractInner<T>,
        T: Clone + Send + Sync,
    {
        #[cfg(feature = "profiler")]
        profiling::scope!("HierarchyChunk::get_all_by_type_mut");
        self.objects
            .par_iter_mut()
            .filter_map(|x| x.obj.extract_inner_mut())
            .collect::<Vec<&mut T>>()
    }

    // #[profiling::function]
    pub fn get_all_by_type_cloned<T>(&self) -> Vec<T>
    where
        HierarchyObjectType: ExtractInner<T>,
        T: Clone + Send + Sync,
    {
        #[cfg(feature = "profiler")]
        profiling::scope!("HierarchyChunk::get_all_by_type_cloned");
        self.objects
            .par_iter()
            .filter_map(|x| x.obj.extract_inner_cloned())
            .collect::<Vec<T>>()
    }

    // pub fn filter_objects<F>(&self, predicate: F) -> Vec<&HierarchyObjectType>
    // where
    //     F: Fn(&HierarchyObjectType) -> bool, // Predicate to filter objects
    // {
    //     self.objects
    //         .iter()
    //         .filter(|x| predicate(&x.obj)) // Apply the predicate
    //         .map(|x| &x.obj) // Extract the object reference
    //         .collect()
    // }

    // #[profiling::function]
    pub fn filter_objects<F, T>(&self, predicate: F) -> Vec<T>
    where
        F: Fn(&T) -> bool + Sync,
        HierarchyObjectType: ExtractInner<T>,
        T: Clone + Send + Sync,
    {
        #[cfg(feature = "profiler")]
        profiling::scope!("HierarchyChunk::filter_objects");
        self.objects
            .par_iter()
            .filter_map(|x| x.obj.extract_inner())
            .filter(|&inner| predicate(inner))
            .cloned()
            .collect::<Vec<T>>()
    }

    // #[profiling::function]
    pub fn filter_objects_mut<F, T>(&mut self, predicate: F) -> Vec<T>
    where
        F: Fn(&T) -> bool + Sync,
        HierarchyObjectType: ExtractInner<T>,
        T: Clone + Send + Sync,
    {
        #[cfg(feature = "profiler")]
        profiling::scope!("HierarchyChunk::filter_objects_mut");
        self.objects
            .par_iter_mut()
            .filter_map(|x| x.obj.extract_inner())
            .filter(|&inner| predicate(inner))
            .cloned()
            .collect::<Vec<T>>()
    }
}

#[derive(BinRead, Debug, Clone)]
pub struct HierarchyObject {
    pub header: HierarchyObjectHeader,
    // #[br(args { h: header.clone() } )]
    #[br(ignore)]
    pub obj: HierarchyObjectType,
    // #[br(if(obj == HierarchyObjectType), pad_after = header.len)]
    // _s: u8,
}

#[derive(BinRead, Debug, Clone)]
pub struct HierarchyObjectHeader {
    pub ty: u8,
    pub len: u32,
}
