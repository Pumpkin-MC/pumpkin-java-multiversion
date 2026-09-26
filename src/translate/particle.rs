//! Particles: id and options.

use pumpkin_data::particle::Particle;
use pumpkin_protocol::{
    VarInt,
    ser::{NetworkReadExt, NetworkWriteExt},
};
use pumpkin_util::version::JavaMinecraftVersion;

use crate::remap::particle_id_remap::remap_particle_id_for_version;
use crate::translate::block::remap_state;

eras! {
    pub enum ParticleFormat {
        /// Particle options before 1.20.5 are not translated.
        Unsupported = V_1_7_2,
        /// Dust colors are three floats.
        V1_20_5 = V_1_20_5,
        /// Dust colors are packed RGB ints.
        V1_21_2 = V_1_21_2,
        /// Effect, instant effect, dragon breath and flash gained options.
        V1_21_9 = V_1_21_9,
    }
}

/// 26.3 option layouts.
enum Options {
    None,
    BlockState,
    Color,
    Dust,
    DustColorTransition,
    Spell,
    Power,
    Float,
    VarInt,
}

/// `None` for options that are not translated yet.
const fn options(particle: Particle) -> Option<Options> {
    use Particle as P;
    Some(match particle {
        P::Block | P::BlockMarker | P::FallingDust | P::DustPillar | P::BlockCrumble => {
            Options::BlockState
        }
        P::EntityEffect | P::TintedLeaves | P::Flash => Options::Color,
        P::Dust => Options::Dust,
        P::DustColorTransition => Options::DustColorTransition,
        P::Effect | P::InstantEffect => Options::Spell,
        P::DragonBreath => Options::Power,
        P::SculkCharge => Options::Float,
        P::Shriek => Options::VarInt,
        // TODO: item, vibration, trail and geyser options
        P::Item | P::Vibration | P::Trail | P::Geyser | P::GeyserBase | P::GeyserPoof
        | P::GeyserPlume => return None,
        _ => Options::None,
    })
}

/// Packed RGB as the three floats of older dust particles.
fn write_rgb_floats(color: i32, out: &mut Vec<u8>) -> Option<()> {
    for shift in [16, 8, 0] {
        out.write_f32_be(((color >> shift) & 0xFF) as f32 / 255.0).ok()?;
    }
    Some(())
}

/// Reads one 26.3 particle and writes it for `version`.
pub fn write_particle(
    read: &mut &[u8],
    version: JavaMinecraftVersion,
    out: &mut Vec<u8>,
) -> Option<()> {
    let format = ParticleFormat::of(version);
    if format == ParticleFormat::Unsupported {
        return None;
    }
    let id = read.get_var_int().ok()?.0;
    let particle = Particle::from_id(u16::try_from(id).ok()?)?;
    let options = options(particle)?;
    out.write_var_int(&VarInt(i32::from(remap_particle_id_for_version(
        id as u16, version,
    ))))
    .ok()?;
    match options {
        Options::None => {}
        Options::BlockState => {
            let state = read.get_var_int().ok()?.0 as u32;
            out.write_var_int(&VarInt(remap_state(state, version) as i32))
                .ok()?;
        }
        Options::Color => {
            let color = read.get_i32_be().ok()?;
            // Flash had no color before 1.21.9
            if particle != Particle::Flash || format >= ParticleFormat::V1_21_9 {
                out.write_i32_be(color).ok()?;
            }
        }
        Options::Dust => {
            let color = read.get_i32_be().ok()?;
            let scale = read.get_f32_be().ok()?;
            if format >= ParticleFormat::V1_21_2 {
                out.write_i32_be(color).ok()?;
            } else {
                write_rgb_floats(color, out)?;
            }
            out.write_f32_be(scale).ok()?;
        }
        Options::DustColorTransition => {
            let from = read.get_i32_be().ok()?;
            let to = read.get_i32_be().ok()?;
            let scale = read.get_f32_be().ok()?;
            if format >= ParticleFormat::V1_21_2 {
                out.write_i32_be(from).ok()?;
                out.write_i32_be(to).ok()?;
            } else {
                write_rgb_floats(from, out)?;
                write_rgb_floats(to, out)?;
            }
            out.write_f32_be(scale).ok()?;
        }
        Options::Spell => {
            let color = read.get_i32_be().ok()?;
            let power = read.get_f32_be().ok()?;
            if format >= ParticleFormat::V1_21_9 {
                out.write_i32_be(color).ok()?;
                out.write_f32_be(power).ok()?;
            }
        }
        Options::Power => {
            let power = read.get_f32_be().ok()?;
            if format >= ParticleFormat::V1_21_9 {
                out.write_f32_be(power).ok()?;
            }
        }
        Options::Float => out.write_f32_be(read.get_f32_be().ok()?).ok()?,
        Options::VarInt => out.write_var_int(&read.get_var_int().ok()?).ok()?,
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dust_color_becomes_floats_before_1_21_2() {
        let mut current = Vec::new();
        current
            .write_var_int(&VarInt(i32::from(Particle::Dust.to_id())))
            .unwrap();
        current.write_i32_be(0x00FF_0000).unwrap();
        current.write_f32_be(2.0).unwrap();

        let mut out = Vec::new();
        write_particle(&mut current.as_slice(), JavaMinecraftVersion::V_1_21, &mut out).unwrap();
        let id = remap_particle_id_for_version(Particle::Dust.to_id(), JavaMinecraftVersion::V_1_21);
        let mut expected = Vec::new();
        expected.write_var_int(&VarInt(i32::from(id))).unwrap();
        for value in [1.0f32, 0.0, 0.0, 2.0] {
            expected.write_f32_be(value).unwrap();
        }
        assert_eq!(out, expected);
    }
}
