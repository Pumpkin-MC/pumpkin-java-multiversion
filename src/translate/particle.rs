//! Particles: id and options, LEVEL_PARTICLES.

use crate::legacy::LegacyWrite;
use pumpkin_data::particle::Particle;
use pumpkin_protocol::{
    VarInt,
    codec::particle::ParticleOptionsLayout,
    java::client::play::CParticle,
    ser::{NetworkReadExt, NetworkWriteExt},
};
use pumpkin_util::{math::vector3::Vector3, version::JavaMinecraftVersion};

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

/// Packed RGB as the three floats of older dust particles.
fn write_rgb_floats(color: i32, out: &mut Vec<u8>) -> Option<()> {
    for shift in [16, 8, 0] {
        out.write_f32_be(((color >> shift) & 0xFF) as f32 / 255.0)
            .ok()?;
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
    let options = ParticleOptionsLayout::of(particle);
    out.write_var_int(&VarInt(i32::from(remap_particle_id_for_version(
        id as u16, version,
    ))))
    .ok()?;
    match options {
        ParticleOptionsLayout::None => {}
        ParticleOptionsLayout::BlockState => {
            let state = read.get_var_int().ok()?.0 as u32;
            out.write_var_int(&VarInt(remap_state(state, version) as i32))
                .ok()?;
        }
        ParticleOptionsLayout::Color => {
            let color = read.get_i32_be().ok()?;
            // Flash had no color before 1.21.9
            if particle != Particle::Flash || format >= ParticleFormat::V1_21_9 {
                out.write_i32_be(color).ok()?;
            }
        }
        ParticleOptionsLayout::Dust => {
            let color = read.get_i32_be().ok()?;
            let scale = read.get_f32_be().ok()?;
            if format >= ParticleFormat::V1_21_2 {
                out.write_i32_be(color).ok()?;
            } else {
                write_rgb_floats(color, out)?;
            }
            out.write_f32_be(scale).ok()?;
        }
        ParticleOptionsLayout::DustColorTransition => {
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
        ParticleOptionsLayout::Spell => {
            let color = read.get_i32_be().ok()?;
            let power = read.get_f32_be().ok()?;
            if format >= ParticleFormat::V1_21_9 {
                out.write_i32_be(color).ok()?;
                out.write_f32_be(power).ok()?;
            }
        }
        ParticleOptionsLayout::Power => {
            let power = read.get_f32_be().ok()?;
            if format >= ParticleFormat::V1_21_9 {
                out.write_f32_be(power).ok()?;
            }
        }
        ParticleOptionsLayout::Float => out.write_f32_be(read.get_f32_be().ok()?).ok()?,
        ParticleOptionsLayout::VarInt => out.write_var_int(&read.get_var_int().ok()?).ok()?,
        // TODO: item, vibration, trail and geyser options
        ParticleOptionsLayout::Item
        | ParticleOptionsLayout::Vibration
        | ParticleOptionsLayout::Trail
        | ParticleOptionsLayout::Geyser
        | ParticleOptionsLayout::GeyserBase => return None,
    }
    Some(())
}

fn read_f32_vec3(read: &mut &[u8]) -> Option<Vector3<f32>> {
    Some(Vector3::new(
        read.get_f32_be().ok()?,
        read.get_f32_be().ok()?,
        read.get_f32_be().ok()?,
    ))
}

/// LEVEL_PARTICLES: 26.3 moved the particle to the front, split the speed per axis, made the
/// count a var int and added a randomization type. Core's writer handles every older layout.
pub fn level_particles_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let mut particle = Vec::new();
    write_particle(&mut payload, version, &mut particle)?;
    let important = payload.get_bool().ok()?;
    let force_spawn = payload.get_bool().ok()?;
    let position = Vector3::new(
        payload.get_f64_be().ok()?,
        payload.get_f64_be().ok()?,
        payload.get_f64_be().ok()?,
    );
    let offset = read_f32_vec3(&mut payload)?;
    let speed = read_f32_vec3(&mut payload)?;
    let count = payload.get_var_int().ok()?.0;
    let _randomization = payload.get_var_int().ok()?;

    let (offset, max_speed) = if speed.x == speed.y && speed.y == speed.z {
        (offset, speed.x)
    } else if count <= 0 {
        // A count of 0 uses offset * speed as the velocity, so fold the speeds into the offsets
        (
            Vector3::new(offset.x * speed.x, offset.y * speed.y, offset.z * speed.z),
            1.0,
        )
    } else {
        // TODO: per axis speeds and the alternative randomization need one packet per
        // particle (ViaBackwards `BlockItemPacketRewriter26_3`); core sends neither yet
        (offset, speed.x)
    };

    let mut particle = particle.as_slice();
    let particle_id = particle.get_var_int().ok()?;
    let mut out = Vec::new();
    CParticle::new(
        force_spawn,
        important,
        position,
        offset,
        max_speed,
        count,
        particle_id,
        particle,
    )
    .write_legacy(&mut out, &version)
    .ok()?;
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::legacy::LegacyWrite;
    use pumpkin_protocol::ClientPacket;

    #[test]
    fn dust_color_becomes_floats_before_1_21_2() {
        let mut current = Vec::new();
        current
            .write_var_int(&VarInt(i32::from(Particle::Dust.to_id())))
            .unwrap();
        current.write_i32_be(0x00FF_0000).unwrap();
        current.write_f32_be(2.0).unwrap();

        let mut out = Vec::new();
        write_particle(
            &mut current.as_slice(),
            JavaMinecraftVersion::V_1_21,
            &mut out,
        )
        .unwrap();
        let id =
            remap_particle_id_for_version(Particle::Dust.to_id(), JavaMinecraftVersion::V_1_21);
        let mut expected = Vec::new();
        expected.write_var_int(&VarInt(i32::from(id))).unwrap();
        for value in [1.0f32, 0.0, 0.0, 2.0] {
            expected.write_f32_be(value).unwrap();
        }
        assert_eq!(out, expected);
    }

    #[test]
    fn level_particles_use_the_26_2_layout() {
        use pumpkin_data::packet::CURRENT_MC_VERSION;

        let flame = VarInt(i32::from(Particle::Flame.to_id()));
        let packet = |id| {
            CParticle::new(
                true,
                false,
                Vector3::new(1.0, 2.0, 3.0),
                Vector3::new(0.1, 0.2, 0.3),
                0.5,
                10,
                id,
                &[],
            )
        };
        let mut current = Vec::new();
        packet(flame)
            .write_packet_data(&mut current, &CURRENT_MC_VERSION)
            .unwrap();

        let version = JavaMinecraftVersion::V_26_2;
        let id = remap_particle_id_for_version(Particle::Flame.to_id(), version);
        let mut expected = Vec::new();
        packet(VarInt(i32::from(id)))
            .write_legacy(&mut expected, &version)
            .unwrap();
        assert_eq!(
            level_particles_from_current(&current, version),
            Some(expected)
        );
    }
}
