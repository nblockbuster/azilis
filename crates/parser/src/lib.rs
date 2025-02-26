#![feature(const_copy_from_slice)]

pub mod hierarchy;
use hierarchy::{music::AudioPathElement, *};

use anyhow::Result;
use binrw::{BinRead, BinReaderExt};
use std::io::Cursor;

macro_rules! fourcc {
    ($bytes:expr) => {{
        let value = u32::from_be_bytes(to_4!($bytes));
        value as isize
    }};
}

macro_rules! to_4 {
    ($str:expr) => {{
        let a = $str.as_bytes();
        let mut b: [u8; 4] = [0u8; 4];
        b.copy_from_slice(&a);
        b
    }};
}

#[derive(BinRead, Debug, Clone, Copy)]
pub struct SoundbankChunkHeader {
    pub name: ChunkName,
    pub len: u32,
}

#[derive(BinRead, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[brw(repr(u32), big)]
pub enum ChunkName {
    BKHD = fourcc!("BKHD"),
    HIRC = fourcc!("HIRC"),
    INIT = fourcc!("INIT"),
    STMG = fourcc!("STMG"),
    STID = fourcc!("STID"),
    PLAT = fourcc!("PLAT"),
    DIDX = fourcc!("DIDX"),
    DATA = fourcc!("DATA"),
    ENVS = fourcc!("ENVS"),
}

#[derive(BinRead, Debug, Clone, Copy)]
pub struct BankHeaderChunk {
    #[br(assert (ver == 0x8C))] // 2021.1
    pub ver: u32,
    pub id: u32,
    pub language_id: u32,
    #[br(pad_after(8))]
    pub project_id: u32,
}

#[derive(BinRead, Debug, Clone)]
#[br(import { ty: ChunkName })]
pub enum SoundbankChunkTypes {
    #[br(pre_assert(ty == ChunkName::BKHD))]
    BankHeader(BankHeaderChunk),
    #[br(pre_assert(ty == ChunkName::HIRC))]
    Hierarchy(HierarchyChunk),
    #[br(pre_assert(ty == ChunkName::INIT))]
    Init,
    #[br(pre_assert(ty == ChunkName::STMG))]
    GlobalSettings,
    #[br(pre_assert(ty == ChunkName::STID))]
    SoundbankIdToString,
    #[br(pre_assert(ty == ChunkName::PLAT))]
    Platform,
    #[br(pre_assert(ty == ChunkName::DIDX))]
    MediaIndex,
    #[br(pre_assert(ty == ChunkName::DATA))]
    Data,
    #[br(pre_assert(ty == ChunkName::ENVS))]
    EnvironmentSettings
}

#[derive(BinRead, Debug, Clone)]
pub struct SoundbankChunk {
    pub header: SoundbankChunkHeader,
    #[br(args { ty: header.name })]
    pub chunk: SoundbankChunkTypes,
}

// #[profiling::function]
pub fn parse(data: &[u8]) -> Result<Vec<SoundbankChunk>> {
    #[cfg(feature = "profiler")]
    profiling::scope!("parser::parse");
    let mut chunks: Vec<SoundbankChunk> = Vec::new();
    let mut cur = Cursor::new(data);
    let c = cur.read_le();
    if c.is_err() {
        return Err(anyhow::anyhow!("Invalid Soundbank Header"));
    }
    chunks.push(c.unwrap());
    let c = cur.read_le();
    if c.is_err() {
        return Err(anyhow::anyhow!("Invalid Soundbank Header"));
    }
    chunks.push(c.unwrap());

    chunks.iter_mut().try_for_each(|c| -> Result<()> {
        match &mut c.chunk {
            SoundbankChunkTypes::Hierarchy(hirc) => {
                hirc.read_hierarchy(&mut cur)?;
                // Generate paths for Music Switch Containers
                hirc.objects
                    .iter_mut()
                    .try_for_each(|f| -> anyhow::Result<()> {
                        if let HierarchyObjectType::MusicSwitchContainer(switch) = &mut f.obj {
                            let path = switch.read_path_element(0)?;
                            if let AudioPathElement::AudioPath(node) = path {
                                switch.paths = node;
                            }
                        }
                        Ok(())
                    })?;
            }
            // TODO: INIT, STMG, DIDX, etc.
            _ => {}
        }

        Ok(())
    })?;

    Ok(chunks)
}
