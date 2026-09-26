//! Explosions: EXPLODE.

use pumpkin_protocol::{
    ClientPacket, IdOr, SoundEvent, VarInt,
    java::client::play::CExplosion,
    ser::{NetworkReadExt, NetworkWriteExt},
};
use pumpkin_util::{math::vector3::Vector3, version::JavaMinecraftVersion};

use crate::translate::{particle::write_particle, sound::read_sound};

eras! {
    pub enum ExplosionFormat {
        /// Older layouts, written by core without particle options or block particles.
        Core = V_1_7_2,
        /// Radius, block count and block particles; 26.3 only adds the play sound flag.
        V1_21_9 = V_1_21_9,
    }
}

fn read_vec3(read: &mut &[u8]) -> Option<Vector3<f64>> {
    Some(Vector3::new(
        read.get_f64_be().ok()?,
        read.get_f64_be().ok()?,
        read.get_f64_be().ok()?,
    ))
}

/// EXPLODE: the client's particle and sound ids. Without the 26.3 play sound flag the sound
/// becomes an empty one, as older clients always play it.
pub fn explode_from_current(mut payload: &[u8], version: JavaMinecraftVersion) -> Option<Vec<u8>> {
    let center = read_vec3(&mut payload)?;
    let radius = payload.get_f32_be().ok()?;
    let block_count = payload.get_i32_be().ok()?;
    let knockback = if payload.get_bool().ok()? {
        Some(read_vec3(&mut payload)?)
    } else {
        None
    };
    let mut particle = Vec::new();
    write_particle(&mut payload, version, &mut particle)?;
    let sound = read_sound(&mut payload, version)?;

    let block_particle_count = payload.get_var_int().ok()?;
    let mut block_particles = Vec::new();
    block_particles.write_var_int(&block_particle_count).ok()?;
    for _ in 0..block_particle_count.0 {
        write_particle(&mut payload, version, &mut block_particles)?;
        block_particles
            .write_f32_be(payload.get_f32_be().ok()?)
            .ok()?; // scaling
        block_particles
            .write_f32_be(payload.get_f32_be().ok()?)
            .ok()?; // speed
        block_particles
            .write_var_int(&payload.get_var_int().ok()?)
            .ok()?; // weight
    }
    let sound = if payload.get_bool().ok()? {
        sound
    } else {
        IdOr::Value(SoundEvent {
            sound_name: "intentionally_empty".into(),
            range: None,
        })
    };

    let mut out = Vec::new();
    if ExplosionFormat::of(version) == ExplosionFormat::Core {
        let particle_id = (&mut particle.as_slice()).get_var_int().ok()?;
        CExplosion {
            center,
            radius,
            block_count,
            knockback,
            particle: particle_id,
            sound,
            block_particles_pool_size: VarInt(0),
        }
        .write_packet_data(&mut out, &version)
        .ok()?;
        return Some(out);
    }
    for value in [center.x, center.y, center.z] {
        out.write_f64_be(value).ok()?;
    }
    out.write_f32_be(radius).ok()?;
    out.write_i32_be(block_count).ok()?;
    out.write_option(&knockback, |w, k| {
        w.write_f64_be(k.x)?;
        w.write_f64_be(k.y)?;
        w.write_f64_be(k.z)
    })
    .ok()?;
    out.extend_from_slice(&particle);
    sound
        .write(&mut out, |w, e| {
            w.write_string(&e.sound_name)?;
            w.write_option(&e.range, |w, r| w.write_f32_be(*r))
        })
        .ok()?;
    out.extend_from_slice(&block_particles);
    Some(out)
}

#[cfg(test)]
mod tests {
    use pumpkin_data::{packet::CURRENT_MC_VERSION, particle::Particle};

    use crate::remap::{
        particle_id_remap::remap_particle_id_for_version,
        sound_id_remap::remap_sound_id_for_version,
    };

    use super::*;

    #[test]
    fn explode_drops_the_play_sound_flag() {
        let version = JavaMinecraftVersion::V_1_21_11;
        let particle = Particle::ExplosionEmitter.to_id();
        let packet = |particle: u16, sound: u16| CExplosion {
            center: Vector3::new(1.0, 2.0, 3.0),
            radius: 4.0,
            block_count: 12,
            knockback: Some(Vector3::new(0.5, 0.25, 0.0)),
            particle: VarInt(i32::from(particle)),
            sound: IdOr::Id(sound),
            block_particles_pool_size: VarInt(0),
        };
        let mut current = Vec::new();
        packet(particle, 300)
            .write_packet_data(&mut current, &CURRENT_MC_VERSION)
            .unwrap();
        let mut expected = Vec::new();
        packet(
            remap_particle_id_for_version(particle, version),
            remap_sound_id_for_version(300, version),
        )
        .write_packet_data(&mut expected, &version)
        .unwrap();
        assert_eq!(explode_from_current(&current, version), Some(expected));
    }
}
