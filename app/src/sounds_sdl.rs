// SPDX-FileCopyrightText: 2024 Janet Blackquill <uhhadd@gmail.com>
//
// SPDX-License-Identifier: MPL-2.0

use logic::{hooks::Sounds, well::Block};
use sdl3::{
    iostream::IOStream,
    mixer::{self, Mixer},
};

pub struct ClientSounds<'mixer> {
    track: mixer::Track<'mixer>,
    lock: mixer::Audio,
    land: mixer::Audio,
    lineclear: mixer::Audio,
    pieces1: mixer::Audio,
    pieces2: mixer::Audio,
    pieces3: mixer::Audio,
    pieces4: mixer::Audio,
    pieces5: mixer::Audio,
    pieces6: mixer::Audio,
    pieces7: mixer::Audio,
}

const LOCK: &'static [u8] = include_bytes!("audio/lock.wav");
const LAND: &'static [u8] = include_bytes!("audio/land.wav");
const LINECLEAR: &'static [u8] = include_bytes!("audio/lineclear.wav");
const PIECES1: &'static [u8] = include_bytes!("audio/pieces1.wav");
const PIECES2: &'static [u8] = include_bytes!("audio/pieces2.wav");
const PIECES3: &'static [u8] = include_bytes!("audio/pieces3.wav");
const PIECES4: &'static [u8] = include_bytes!("audio/pieces4.wav");
const PIECES5: &'static [u8] = include_bytes!("audio/pieces5.wav");
const PIECES6: &'static [u8] = include_bytes!("audio/pieces6.wav");
const PIECES7: &'static [u8] = include_bytes!("audio/pieces7.wav");

impl<'mixer> Sounds for ClientSounds<'mixer> {
    fn line_clear(&mut self) {
        self.track.set_audio(&self.lineclear).unwrap();
        self.track.play().unwrap();
    }
    fn block_spawn(&mut self, color: Block) {
        self.track
            .set_audio(match color {
                Block::Yellow => &self.pieces1,
                Block::Blue => &self.pieces2,
                Block::Orange => &self.pieces3,
                Block::Green => &self.pieces4,
                Block::Purple => &self.pieces5,
                Block::Cyan => &self.pieces6,
                Block::Red => &self.pieces7,
            })
            .unwrap();

        self.track.play().unwrap();
    }
    fn lock(&mut self) {
        self.track.set_audio(&self.lock).unwrap();
        self.track.play().unwrap();
    }
    fn land(&mut self) {
        self.track.set_audio(&self.land).unwrap();
        self.track.play().unwrap();
    }
}

impl<'mixer> ClientSounds<'mixer> {
    pub fn new(mix: &'mixer Mixer) -> Result<ClientSounds<'mixer>, sdl3::Error> {
        Ok(ClientSounds {
            track: mix.create_track()?,
            lock: mix.load_audio_io(&IOStream::from_bytes(LOCK)?, false)?,
            land: mix.load_audio_io(&IOStream::from_bytes(LAND)?, false)?,
            lineclear: mix.load_audio_io(&IOStream::from_bytes(LINECLEAR)?, false)?,
            pieces1: mix.load_audio_io(&IOStream::from_bytes(PIECES1)?, false)?,
            pieces2: mix.load_audio_io(&IOStream::from_bytes(PIECES2)?, false)?,
            pieces3: mix.load_audio_io(&IOStream::from_bytes(PIECES3)?, false)?,
            pieces4: mix.load_audio_io(&IOStream::from_bytes(PIECES4)?, false)?,
            pieces5: mix.load_audio_io(&IOStream::from_bytes(PIECES5)?, false)?,
            pieces6: mix.load_audio_io(&IOStream::from_bytes(PIECES6)?, false)?,
            pieces7: mix.load_audio_io(&IOStream::from_bytes(PIECES7)?, false)?,
        })
    }
}
