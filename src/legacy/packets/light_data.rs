//! Pre-26.3 encodings from `pumpkin-protocol`'s `java/client/play/light_update.rs`.

use crate::legacy::{LegacyWrite, write_bit_set_legacy};
use pumpkin_protocol::java::client::play::LightData;
use pumpkin_protocol::{
    VarInt,
    codec::bit_set::BitSet,
    ser::{NetworkWriteExt, WritingError},
};
use pumpkin_util::version::JavaMinecraftVersion;
use std::io::Write;

/// Sections a pre-1.17 mask covers: the world plus one below and above.
const LEGACY_SECTIONS: usize = 18;

impl LegacyWrite for LightData {
    fn write_legacy(
        &self,
        mut write: impl Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), WritingError> {
        if (JavaMinecraftVersion::V_1_16..=JavaMinecraftVersion::V_1_19_4).contains(version) {
            write.write_bool(self.trust_edges)?;
        }
        let masks = [
            &self.sky_light_mask,
            &self.block_light_mask,
            &self.empty_sky_light_mask,
            &self.empty_block_light_mask,
        ];
        if *version >= JavaMinecraftVersion::V_1_17 {
            for mask in masks {
                write_bit_set_legacy(&mut write, mask, version)?;
            }
            write_arrays(&mut write, &self.sky_light_arrays)?;
            write_arrays(&mut write, &self.block_light_arrays)
        } else {
            for mask in masks {
                write.write_var_int(&VarInt(mask.as_u64() as i32))?;
            }
            write_masked_arrays(&mut write, &self.sky_light_mask, &self.sky_light_arrays)?;
            write_masked_arrays(&mut write, &self.block_light_mask, &self.block_light_arrays)
        }
    }
}

/// Count, then each array with its length.
fn write_arrays(write: &mut impl Write, arrays: &[Vec<u8>]) -> Result<(), WritingError> {
    write.write_var_int(&VarInt(arrays.len() as i32))?;
    for array in arrays {
        write.write_var_int(&VarInt(array.len() as i32))?;
        write.write_slice(array)?;
    }
    Ok(())
}

/// Before 1.17: no count, one array per set mask bit.
fn write_masked_arrays(
    write: &mut impl Write,
    mask: &BitSet,
    arrays: &[Vec<u8>],
) -> Result<(), WritingError> {
    let mut arrays = arrays.iter();
    for section in 0..LEGACY_SECTIONS {
        if mask.get_bit(section)
            && let Some(array) = arrays.next()
        {
            write.write_var_int(&VarInt(array.len() as i32))?;
            write.write_slice(array)?;
        }
    }
    Ok(())
}
