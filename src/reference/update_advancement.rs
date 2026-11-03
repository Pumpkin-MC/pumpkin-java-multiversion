//! Pre-26.3 encoding from `pumpkin-protocol`'s `java/client/play/update_advancement.rs`, as a
//! test reference for 1.20.5 and later (older item layouts are not reproduced).

use pumpkin_protocol::{
    VarInt,
    codec::item_stack_seralizer::{ItemStackSerializer, ItemStackTemplateSerializer},
    java::client::play::CUpdateAdvancements,
    ser::{NetworkWriteExt, WritingError},
};
use pumpkin_util::version::JavaMinecraftVersion;

use super::ReferenceWrite;
use pumpkin_protocol::java::legacy::LegacyWriteExt;

impl ReferenceWrite for CUpdateAdvancements {
    fn write_legacy(
        &self,
        mut write: impl std::io::Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), WritingError> {
        write.write_bool(self.reset)?;

        write.write_var_int(&VarInt(self.added.len() as i32))?;
        for adv in &self.added {
            write.write_string(&adv.id.to_string())?;

            let has_parent = adv.parent.is_some();
            write.write_bool(has_parent)?;
            if let Some(ref p) = adv.parent {
                write.write_string(&p.to_string())?;
            }

            let has_display = adv.display.is_some();
            write.write_bool(has_display)?;
            if let Some(display) = adv.display {
                write.write_component_legacy(&display.get_title(), version)?;
                write.write_component_legacy(&display.get_description(), version)?;

                // Item icon
                // Before 26.1 the icon was a full stack; from 1.20.5 on its layout is core's.
                if *version < JavaMinecraftVersion::V_26_1 {
                    ItemStackSerializer::from(display.item_icon.clone()).write(&mut write)?;
                } else {
                    ItemStackTemplateSerializer::from(display.item_icon.clone())
                        .write(&mut write)?;
                }

                write.write_var_int(&VarInt(display.frame_type as i32))?;
                let flags = (display.has_background() as i32)
                    | ((display.show_toast as i32) << 1)
                    | ((display.hidden as i32) << 2);
                write.write_i32_be(flags)?;
                if let Some(bg) = display.background_texture {
                    write.write_string(bg)?;
                }
                if *version < JavaMinecraftVersion::V_26_3 {
                    write.write_f32_be(display.x)?;
                    write.write_f32_be(display.y)?;
                }
            }

            if *version < JavaMinecraftVersion::V_1_20_2 {
                write.write_var_int(&VarInt(adv.criteria.len() as i32))?;
                for crit in adv.criteria {
                    write.write_string(crit)?;
                }
            }

            write.write_var_int(&VarInt(adv.requirements.len() as i32))?;
            for req in adv.requirements {
                write.write_var_int(&VarInt(req.len() as i32))?;
                for r in *req {
                    write.write_string(r)?;
                }
            }

            if *version >= JavaMinecraftVersion::V_1_20 {
                write.write_bool(adv.send_telemetry)?;
            }

            // Since 26.3 the position in the advancement tree is sent per advancement instead of
            // being part of its display.
            if *version >= JavaMinecraftVersion::V_26_3 {
                let (x, y) = adv.display.map_or((0.0, 0.0), |d| (d.x, d.y));
                write.write_f32_be(x)?;
                write.write_f32_be(y)?;
            }
        }

        write.write_var_int(&VarInt(self.removed.len() as i32))?;
        for rem in &self.removed {
            write.write_string(&rem.to_string())?;
        }

        write.write_var_int(&VarInt(self.progress.len() as i32))?;
        for prog in &self.progress {
            write.write_string(&prog.id.to_string())?;
            write.write_var_int(&VarInt(prog.progress.len() as i32))?;
            for crit in &prog.progress {
                write.write_string(&crit.criterion_id)?;
                let has_date = crit.achieve_date.is_some();
                write.write_bool(has_date)?;
                if let Some(date) = crit.achieve_date {
                    write.write_i64_be(date)?;
                }
            }
        }

        if *version >= JavaMinecraftVersion::V_1_21_5 {
            write.write_bool(self.show_advancements)?;
        }

        Ok(())
    }
}
