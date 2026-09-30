//! Light data: LIGHT_UPDATE and the tail of LEVEL_CHUNK_WITH_LIGHT.

use pumpkin_protocol::{
    codec::bit_set::BitSet,
    ser::{NetworkReadExt, NetworkWriteExt},
};
use pumpkin_util::version::JavaMinecraftVersion;

eras! {
    pub enum LightFormat {
        /// Var int masks, arrays without count.
        V1_14 = V_1_7_2,
        /// Trust edges flag added.
        V1_16 = V_1_16,
        /// Masks as long arrays, counted light arrays.
        V1_17 = V_1_17,
        /// Trust edges flag removed.
        V1_20 = V_1_20,
        /// Masks as byte arrays (26.3).
        V26_3 = V_26_3,
    }
}

/// Current light data (four masks, sky and block arrays) for `version`.
// TODO: V1_14 and V1_16 masks and arrays.
pub fn write_light_data(
    mut light: &[u8],
    version: JavaMinecraftVersion,
    out: &mut Vec<u8>,
) -> Option<()> {
    let format = LightFormat::of(version);
    match format {
        LightFormat::V1_14 | LightFormat::V1_20 | LightFormat::V26_3 => {}
        LightFormat::V1_16 | LightFormat::V1_17 => out.push(1),
    }
    // sky, block, empty sky, empty block
    for _ in 0..4 {
        let mask = BitSet::decode(&mut light).ok()?;
        crate::legacy::write_bit_set_legacy(&mut *out, &mask, &version).ok()?;
    }
    out.extend_from_slice(light);
    Some(())
}

/// LIGHT_UPDATE: chunk x, z, light data.
pub fn light_update_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let x = payload.get_var_int().ok()?;
    let z = payload.get_var_int().ok()?;
    let mut out = Vec::new();
    out.write_var_int(&x).ok()?;
    out.write_var_int(&z).ok()?;
    write_light_data(payload, version, &mut out)?;
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_become_long_arrays_before_26_3() {
        let mask = BitSet(Box::new([0b1011_0000_0001]));
        let mut light = Vec::new();
        for _ in 0..4 {
            mask.encode(&mut light).unwrap();
        }
        let masks_len = light.len();
        // one sky array, no block arrays
        light.write_var_int(&pumpkin_protocol::VarInt(1)).unwrap();
        light
            .write_var_int(&pumpkin_protocol::VarInt(2048))
            .unwrap();
        light.extend_from_slice(&[0x11; 2048]);
        light.write_var_int(&pumpkin_protocol::VarInt(0)).unwrap();

        let mut expected = Vec::new();
        for _ in 0..4 {
            crate::legacy::write_bit_set_legacy(
                &mut expected,
                &mask,
                &JavaMinecraftVersion::V_1_21_11,
            )
            .unwrap();
        }
        expected.extend_from_slice(&light[masks_len..]);

        let mut out = Vec::new();
        write_light_data(&light, JavaMinecraftVersion::V_1_21_11, &mut out).unwrap();
        assert_eq!(out, expected);
    }
}
