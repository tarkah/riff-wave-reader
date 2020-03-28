use byteorder::{ByteOrder, LittleEndian};

use std::io::Read;
use std::io::Seek;
use std::mem;
use std::{io::SeekFrom, marker::PhantomData};

mod error;
pub use error::Error;

#[derive(Debug)]
pub struct RiffWaveReader<T: Read + Seek> {
    reader: T,
    pub riff_chunk: RiffChunk,
    pub fmt_chunk: FmtChunk,
    pub fact_chunk: Option<FactChunk>,
    pub data_chunk: DataChunk,
    pub other_chunks: Vec<OtherChunk>,
}

impl<T: Read + Seek> RiffWaveReader<T> {
    pub fn new(mut reader: T) -> Result<RiffWaveReader<T>, Error> {
        let riff_chunk = reader.read_riff_chunk()?;

        if riff_chunk.id != FourCC::Riff {
            return Err(Error::NotRiff);
        }

        if riff_chunk.file_type != FourCC::Wave {
            return Err(Error::NotWave);
        }

        let fmt_chunk = reader.read_fmt_chunk()?;

        let fact_chunk = reader.read_fact_chunk()?;

        let mut other_chunks = vec![];
        reader.read_other_chunks(&mut other_chunks)?;

        let data_chunk = reader.read_data_chunk()?;

        let riff_reader = RiffWaveReader {
            reader,
            riff_chunk,
            fmt_chunk,
            fact_chunk,
            data_chunk,
            other_chunks,
        };

        Ok(riff_reader)
    }

    pub fn data(&mut self) -> Result<impl Iterator<Item = u8>, Error> {
        let mut data = vec![];
        self.reader.read_to_end(&mut data)?;

        Ok(data.into_iter())
    }

    pub fn print_info(&self) {
        println!("{}", self);
    }
}

trait ReadExt: Read + Seek {
    fn read_riff_chunk(&mut self) -> Result<RiffChunk, Error>;

    fn read_fmt_chunk(&mut self) -> Result<FmtChunk, Error>;

    fn read_extended_info(&mut self, size: u16) -> Result<Option<ExtendedInfo>, Error>;

    fn read_fact_chunk(&mut self) -> Result<Option<FactChunk>, Error>;

    fn read_other_chunks(&mut self, other_chunks: &mut Vec<OtherChunk>) -> Result<(), Error>;

    fn read_data_chunk(&mut self) -> Result<DataChunk, Error>;

    fn read_fourcc(&mut self) -> Result<FourCC, Error>;

    fn read_u32(&mut self) -> Result<u32, Error>;

    fn read_u16(&mut self) -> Result<u16, Error>;

    fn read_u128(&mut self) -> Result<u128, Error>;

    fn read_is_fourcc(&mut self) -> Result<bool, Error>;
}

impl<T: Read + Seek> ReadExt for T {
    fn read_riff_chunk(&mut self) -> Result<RiffChunk, Error> {
        let id = self.read_fourcc()?;
        let file_size = self.read_u32()?;
        let file_type = self.read_fourcc()?;

        Ok(RiffChunk {
            id,
            file_size,
            file_type,
        })
    }

    fn read_fmt_chunk(&mut self) -> Result<FmtChunk, Error> {
        let id = self.read_fourcc()?;
        if id != FourCC::Fmt {
            return Err(Error::InvalidFmtChunk);
        }

        let data_size = self.read_u32()?;
        let format = Format::from(self.read_u16()?);
        let num_channels = self.read_u16()?;
        let sample_rate = self.read_u32()?;
        let byte_rate = self.read_u32()?;
        let block_align = self.read_u16()?;
        let bits_per_sample = self.read_u16()?;

        let (extra_info_size, extended_info) = if self.read_is_fourcc()? {
            (0, None)
        } else {
            let extra_info_size = self.read_u16()?;
            (extra_info_size, self.read_extended_info(extra_info_size)?)
        };

        Ok(FmtChunk {
            id,
            data_size,
            format,
            num_channels,
            sample_rate,
            byte_rate,
            block_align,
            bits_per_sample,
            extra_info_size,
            extended_info,
        })
    }

    fn read_extended_info(&mut self, size: u16) -> Result<Option<ExtendedInfo>, Error> {
        if size == 0 {
            return Ok(None);
        }

        if size < 22 {
            return Err(Error::InvalidExtendedInfo);
        }

        let sample_info = self.read_u16()?;
        let channel_mask = self.read_u32()?;
        let sub_format = self.read_u128()?;

        let remaining_size = (size - 22) as usize;
        let mut remaining_data = vec![0; remaining_size];
        self.read_exact(&mut remaining_data[..])?;

        Ok(Some(ExtendedInfo {
            sample_info,
            channel_mask,
            sub_format,
            remaining_data,
        }))
    }

    fn read_fact_chunk(&mut self) -> Result<Option<FactChunk>, Error> {
        let id = self.read_fourcc()?;
        if id != FourCC::Fact {
            self.seek(SeekFrom::Current(-4))?;
            return Ok(None);
        }

        let data_size = self.read_u32()?;
        let sample_length = self.read_u32()?;

        let remaining_size = (data_size - 4) as usize;
        let mut remaining_data = vec![0; remaining_size];
        self.read_exact(&mut remaining_data[..])?;

        Ok(Some(FactChunk {
            id,
            data_size,
            sample_length,
            remaining_data,
        }))
    }

    fn read_other_chunks(&mut self, other_chunks: &mut Vec<OtherChunk>) -> Result<(), Error> {
        loop {
            let fourcc = self.read_fourcc()?;

            if fourcc == FourCC::Data {
                self.seek(SeekFrom::Current(-4))?;
                return Ok(());
            }

            let data_size = self.read_u32()?;
            let mut data = vec![0; data_size as usize];
            self.read_exact(&mut data)?;

            let chunk = OtherChunk {
                id: fourcc,
                data_size,
                data,
            };

            other_chunks.push(chunk);
        }
    }

    fn read_data_chunk(&mut self) -> Result<DataChunk, Error> {
        let id = self.read_fourcc()?;
        let data_size = self.read_u32()?;

        let pad_byte = if data_size % 2 == 0 { 0 } else { 1 };

        Ok(DataChunk {
            id,
            data_size,
            pad_byte,
        })
    }

    fn read_fourcc(&mut self) -> Result<FourCC, Error> {
        let mut buf = [0; 4];

        self.read_exact(&mut buf)?;

        Ok(FourCC::from(&buf[..]))
    }

    fn read_u32(&mut self) -> Result<u32, Error> {
        let mut buf = [0; 4];

        self.read_exact(&mut buf)?;

        Ok(LittleEndian::read_u32(&buf))
    }

    fn read_u16(&mut self) -> Result<u16, Error> {
        let mut buf = [0; 2];

        self.read_exact(&mut buf)?;

        Ok(LittleEndian::read_u16(&buf))
    }

    fn read_u128(&mut self) -> Result<u128, Error> {
        let mut buf = [0; 16];

        self.read_exact(&mut buf)?;

        Ok(LittleEndian::read_u128(&buf))
    }

    fn read_is_fourcc(&mut self) -> Result<bool, Error> {
        let fourcc = self.read_fourcc()?;
        self.seek(SeekFrom::Current(-4))?;

        Ok(if let FourCC::Other(_) = fourcc {
            false
        } else {
            true
        })
    }
}

#[derive(Debug)]
pub struct RiffChunk {
    pub id: FourCC,
    pub file_size: u32,
    pub file_type: FourCC,
}

#[derive(Debug)]
pub struct FmtChunk {
    pub id: FourCC,
    pub data_size: u32,
    pub format: Format,
    pub num_channels: u16,
    pub sample_rate: u32,
    pub byte_rate: u32,
    pub block_align: u16,
    pub bits_per_sample: u16,
    pub extra_info_size: u16,
    pub extended_info: Option<ExtendedInfo>,
}

#[derive(Debug)]
pub struct ExtendedInfo {
    pub sample_info: u16,
    pub channel_mask: u32,
    pub sub_format: u128,
    pub remaining_data: Vec<u8>,
}

#[derive(Debug)]
pub struct FactChunk {
    pub id: FourCC,
    pub data_size: u32,
    pub sample_length: u32,
    pub remaining_data: Vec<u8>,
}

#[derive(Debug)]
pub struct OtherChunk {
    pub id: FourCC,
    pub data_size: u32,
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub struct DataChunk {
    pub id: FourCC,
    pub data_size: u32,
    pub pad_byte: u8,
}

#[derive(Debug, PartialEq, Clone)]
pub enum FourCC {
    Riff,
    Fmt,
    Data,
    Wave,
    Fact,
    Other(String),
}

impl From<&[u8]> for FourCC {
    #[allow(clippy::unreadable_literal)]
    fn from(data: &[u8]) -> Self {
        match data {
            b"RIFF" => FourCC::Riff,
            b"WAVE" => FourCC::Wave,
            b"fmt " => FourCC::Fmt,
            b"data" => FourCC::Data,
            b"Data" => FourCC::Data,
            b"fact" => FourCC::Fact,
            _ => {
                let fourcc = unsafe { std::str::from_utf8_unchecked(&data) };
                FourCC::Other(fourcc.to_owned())
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Format {
    UncompressedPCM,
    IeeeFloatingPoint,
    G711ALaw,
    G711ULaw,
    ExtendedWave,
    Other(u16),
}

impl From<u16> for Format {
    fn from(format: u16) -> Self {
        match format {
            1 => Format::UncompressedPCM,
            3 => Format::IeeeFloatingPoint,
            6 => Format::G711ALaw,
            7 => Format::G711ULaw,
            65534 => Format::ExtendedWave,
            _ => Format::Other(format),
        }
    }
}

impl<T: Read + Seek> std::fmt::Display for RiffWaveReader<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let size = self.riff_chunk.file_size;
        let format = self.fmt_chunk.format;
        let num_channels = self.fmt_chunk.num_channels;
        let sample_rate = self.fmt_chunk.sample_rate;
        let byte_rate = self.fmt_chunk.byte_rate;
        let block_align = self.fmt_chunk.block_align;
        let bits_per_sample = self.fmt_chunk.bits_per_sample;
        let extra_info_size = self.fmt_chunk.extra_info_size;

        let extended = if let Some(extended) = &self.fmt_chunk.extended_info {
            format!(
                "\n----- Extended -----
Sample Info:     {}
Channel Mask:    {:#018b}
Sub Format:      {:x}
Remaining Data:  {:?}",
                extended.sample_info,
                extended.channel_mask,
                extended.sub_format,
                &extended.remaining_data[..],
            )
        } else {
            String::from("")
        };

        let fact = if let Some(fact) = &self.fact_chunk {
            format!(
                "\n------- Fact -------
Fact Length:     {}
Sample Length:   {}
Remaining Data:  {:?}",
                fact.data_size,
                fact.sample_length,
                &fact.remaining_data[..],
            )
        } else {
            String::from("")
        };

        let other_chunks = {
            let chunk_ids = self
                .other_chunks
                .iter()
                .map(|c| {
                    if let FourCC::Other(id) = &c.id {
                        id.clone()
                    } else {
                        String::from("")
                    }
                })
                .collect::<Vec<_>>();

            if chunk_ids.is_empty() {
                String::from("")
            } else {
                format!(
                    "\n--- Other Chunks ---
Chunk Ids:       {:?}",
                    chunk_ids
                )
            }
        };

        let data = format!(
            "\n------- Data -------
Data Length:     {}
Padding Byte:    {}",
            self.data_chunk.data_size, self.data_chunk.pad_byte
        );

        write!(
            f,
            "------ Header ------
Size:            {}
Format:          {:?}
Channels:        {}
Sample Rate:     {}
Byte Rate:       {}
Block Align:     {}
Bits per Sample: {}
Extra Info:      {}{}{}{}{}",
            size,
            format,
            num_channels,
            sample_rate,
            byte_rate,
            block_align,
            bits_per_sample,
            extra_info_size,
            extended,
            fact,
            other_chunks,
            data
        )
    }
}

// 0xEABB6C5F8000D1B411CFDB46E06D802C
// 0x62CEE401FAFF19A14471CB58E923AAB4

// { AV_CODEC_ID_AC3,      { 0x2C, 0x80, 0x6D, 0xE0, 0x46, 0xDB, 0xCF, 0x11, 0xB4, 0xD1, 0x00, 0x80, 0x5F, 0x6C, 0xBB, 0xEA } },
// { AV_CODEC_ID_ATRAC3P,  { 0xBF, 0xAA, 0x23, 0xE9, 0x58, 0xCB, 0x71, 0x44, 0xA1, 0x19, 0xFF, 0xFA, 0x01, 0xE4, 0xCE, 0x62 } },
// { AV_CODEC_ID_ATRAC9,   { 0xD2, 0x42, 0xE1, 0x47, 0xBA, 0x36, 0x8D, 0x4D, 0x88, 0xFC, 0x61, 0x65, 0x4F, 0x8C, 0x83, 0x6C } },
// { AV_CODEC_ID_EAC3,     { 0xAF, 0x87, 0xFB, 0xA7, 0x02, 0x2D, 0xFB, 0x42, 0xA4, 0xD4, 0x05, 0xCD, 0x93, 0x84, 0x3B, 0xDD } },
// { AV_CODEC_ID_MP2,      { 0x2B, 0x80, 0x6D, 0xE0, 0x46, 0xDB, 0xCF, 0x11, 0xB4, 0xD1, 0x00, 0x80, 0x5F, 0x6C, 0xBB, 0xEA } },
// { AV_CODEC_ID_ADPCM_AGM,{ 0x82, 0xEC, 0x1F, 0x6A, 0xCA, 0xDB, 0x19, 0x45, 0xBD, 0xE7, 0x56, 0xD3, 0xB3, 0xEF, 0x98, 0x1D } },