use pumpkin_util::version::JavaMinecraftVersion;

use crate::codec::var_int::VarIntType;

pub trait Packet {
    const PACKET_ID: VarIntType;
}

pub trait MultiVersionJavaPacket {
    #[must_use]
    fn to_id(version: JavaMinecraftVersion) -> i32;
}
