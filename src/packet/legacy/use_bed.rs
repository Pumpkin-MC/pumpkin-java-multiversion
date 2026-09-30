use crate::legacy::{LegacyReadExt, LegacyWriteExt};
use std::io::Write;

use crate::packet::mappings::clientbound::play::USE_BED;
use pumpkin_protocol::{
    ClientPacket, MultiVersionJavaPacket, ServerPacket, VarInt,
    ser::{NetworkReadExt, NetworkWriteExt, ReadingError, WritingError},
};
use pumpkin_util::{math::position::BlockPos, version::JavaMinecraftVersion};

/// Sent by the server to tell a client that a player is in bed.
///
/// This packet was used in 1.13.2 and older versions. In 1.14+, it was removed
/// and sleeping is indicated via entity metadata / pose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CUseBed {
    /// The entity ID of the sleeping player.
    pub entity_id: VarInt,
    /// The block position of the head of the bed.
    pub location: BlockPos,
}

impl MultiVersionJavaPacket for CUseBed {
    fn to_id(version: JavaMinecraftVersion) -> i32 {
        USE_BED.to_id(version)
    }
}

impl CUseBed {
    #[must_use]
    pub const fn new(entity_id: VarInt, location: BlockPos) -> Self {
        Self {
            entity_id,
            location,
        }
    }
}

impl ClientPacket for CUseBed {
    fn write_packet_data(
        &self,
        mut write: impl Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), WritingError> {
        if *version >= JavaMinecraftVersion::V_1_8 {
            write.write_var_int(&self.entity_id)?;
            write.write_block_pos_legacy(&self.location, version)?;
        } else {
            write.write_i32_be(self.entity_id.0)?;
            write.write_i32_be(self.location.0.x)?;
            write.write_u8(self.location.0.y as u8)?;
            write.write_i32_be(self.location.0.z)?;
        }

        Ok(())
    }
}

impl<'a> ServerPacket<'a> for CUseBed {
    fn read(read: &mut &'a [u8], version: &JavaMinecraftVersion) -> Result<Self, ReadingError> {
        if *version >= JavaMinecraftVersion::V_1_8 {
            let entity_id = read.get_var_int()?;
            let location = read.get_block_pos_legacy(version)?;
            Ok(Self {
                entity_id,
                location,
            })
        } else {
            let entity_id = VarInt(read.get_i32_be()?);
            let x = read.get_i32_be()?;
            let y = i32::from(read.get_u8()?);
            let z = read.get_i32_be()?;
            let location = BlockPos::new(x, y, z);
            Ok(Self {
                entity_id,
                location,
            })
        }
    }
}
