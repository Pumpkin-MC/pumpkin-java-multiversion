//! Reference encoders of older layouts that the byte translators are tested against.

mod commands;
mod encryption_response;
mod update_advancement;

use pumpkin_protocol::ser::WritingError;
use pumpkin_util::version::JavaMinecraftVersion;

/// A packet encoded the way an older client expects, for comparison in tests.
pub trait ReferenceWrite {
    fn write_legacy(
        &self,
        write: impl std::io::Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), WritingError>;
}
