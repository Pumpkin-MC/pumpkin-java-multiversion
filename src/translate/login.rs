//! Login state: HELLO, KEY and LOGIN_FINISHED.

use pumpkin_protocol::java::legacy::LegacyRead;
use pumpkin_protocol::{
    ClientPacket, Property,
    java::{client::login::CLoginSuccess, server::login::SEncryptionResponse},
    ser::{NetworkReadExt, NetworkReadSliceExt, NetworkWriteExt},
};
use pumpkin_util::version::JavaMinecraftVersion;

eras! {
    pub enum KeyFormat {
        /// i16 length prefixes.
        V1_7 = V_1_7_2,
        /// Var int length prefixes; same as 26.3.
        V1_8 = V_1_8,
        /// Verify token may be replaced by a salted signature.
        V1_19 = V_1_19,
        /// Same as 26.3.
        V1_19_3 = V_1_19_3,
    }
}

eras! {
    pub enum HelloFormat {
        /// Name only.
        V1_7 = V_1_7_2,
        /// Name and UUID; same as 26.3.
        V1_20_2 = V_1_20_2,
    }
}

/// KEY / ENCRYPTION_RESPONSE.
pub fn key_to_current(mut payload: &[u8], version: JavaMinecraftVersion) -> Option<Vec<u8>> {
    match KeyFormat::of(version) {
        KeyFormat::V1_7 | KeyFormat::V1_19 => {}
        KeyFormat::V1_8 | KeyFormat::V1_19_3 => return None,
    }
    let packet = SEncryptionResponse::read_legacy(&mut payload, &version).ok()?;
    let mut out = Vec::new();
    packet.write_packet_data(&mut out).ok()?;
    Some(out)
}

/// HELLO / LOGIN_START: an offline UUID for clients that send none.
pub fn hello_to_current(mut payload: &[u8], version: JavaMinecraftVersion) -> Option<Vec<u8>> {
    if HelloFormat::of(version) == HelloFormat::V1_20_2 {
        return None;
    }
    let name = payload.get_str_borrowed().ok()?;
    let offline_uuid = uuid::Uuid::new_v3(
        &uuid::Uuid::nil(),
        format!("OfflinePlayer:{name}").as_bytes(),
    );
    let mut out = Vec::new();
    let _ = out.write_string(name);
    let _ = out.write_uuid(&offline_uuid);
    Some(out)
}

/// LOGIN_FINISHED / GAME_PROFILE (UUID string before 1.16, properties from 1.19, session from 26.2).
pub fn login_success_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let uuid = payload.get_uuid().ok()?;
    let username = payload.get_str_borrowed().ok()?;
    let properties = payload.get_list(Property::read).ok()?;
    let session_id = payload.get_uuid().ok()?;
    let packet = CLoginSuccess::new(&uuid, username, &properties, true, session_id);
    super::legacy_bytes(&packet, version)
}

#[cfg(test)]
mod tests {
    use crate::reference::ReferenceWrite;
    use pumpkin_plugin_api::events_wit::ConnectionState;
    use pumpkin_protocol::ServerPacket;
    use pumpkin_protocol::{
        VarInt,
        java::client::login::CEncryptionRequest,
        ser::{NetworkReadSliceExt, NetworkWriteExt},
    };

    use super::*;
    use crate::packet::translator::PacketTranslator;
    use pumpkin_protocol::java::legacy::ids;

    fn key_payload(version: JavaMinecraftVersion, secret: &[u8], token: &[u8]) -> Vec<u8> {
        let packet = SEncryptionResponse {
            shared_secret: secret.to_vec().into_boxed_slice(),
            verify_token: token.to_vec().into_boxed_slice(),
        };
        let mut buf = Vec::new();
        packet.write_legacy(&mut buf, &version).unwrap();
        buf
    }

    fn translate_key(version: JavaMinecraftVersion, payload: &[u8]) -> Option<(i32, Vec<u8>)> {
        PacketTranslator::translate_connection_incoming(
            ConnectionState::Login,
            ids::serverbound::login::KEY.to_id(version),
            payload,
            version,
        )
    }

    fn read_current(payload: &[u8]) -> SEncryptionResponse {
        let mut slice = payload;
        SEncryptionResponse::read(&mut slice).unwrap()
    }

    #[test]
    fn key_1_7_i16_lengths_become_varint() {
        let secret = b"secret12secret12";
        let token = b"tokn";
        let payload = key_payload(JavaMinecraftVersion::V_1_7_6, secret, token);
        let (id, out) = translate_key(JavaMinecraftVersion::V_1_7_6, &payload).unwrap();
        assert_eq!(id, ids::serverbound::login::KEY.current());
        let read = read_current(&out);
        assert_eq!(&*read.shared_secret, secret);
        assert_eq!(&*read.verify_token, token);
    }

    #[test]
    fn key_1_19_1_omitted_token_becomes_empty() {
        let secret = b"secret12secret12";
        let signature = b"signature-bytes!!";
        let mut payload = Vec::new();
        payload.write_var_int(&VarInt(secret.len() as i32)).unwrap();
        payload.extend_from_slice(secret);
        payload.write_bool(false).unwrap();
        payload.write_i64_be(0x1122_3344_5566_7788).unwrap();
        payload
            .write_var_int(&VarInt(signature.len() as i32))
            .unwrap();
        payload.extend_from_slice(signature);

        let (id, out) = translate_key(JavaMinecraftVersion::V_1_19_1, &payload).unwrap();
        assert_eq!(id, ids::serverbound::login::KEY.current());
        let read = read_current(&out);
        assert_eq!(&*read.shared_secret, secret);
        assert!(read.verify_token.is_empty());
    }

    #[test]
    fn key_1_19_1_with_token_keeps_token() {
        let secret = b"secret12secret12";
        let token = b"tokn";
        let payload = key_payload(JavaMinecraftVersion::V_1_19_1, secret, token);
        let (_, out) = translate_key(JavaMinecraftVersion::V_1_19_1, &payload).unwrap();
        let read = read_current(&out);
        assert_eq!(&*read.shared_secret, secret);
        assert_eq!(&*read.verify_token, token);
    }

    #[test]
    fn key_1_8_and_1_19_3_payloads_are_already_current() {
        let secret = b"secret12secret12";
        let token = b"tokn";
        for version in [JavaMinecraftVersion::V_1_8, JavaMinecraftVersion::V_1_19_3] {
            let payload = key_payload(version, secret, token);
            let (id, out) = translate_key(version, &payload).unwrap();
            assert_eq!(id, ids::serverbound::login::KEY.current());
            assert_eq!(out, payload);
        }
    }

    #[test]
    fn hello_outgoing_1_7_uses_i16_lengths() {
        let packet = CEncryptionRequest::new("", b"public_key_bytes", b"tokn", true);
        let mut payload = Vec::new();
        packet.write_packet_data(&mut payload).unwrap();
        let (id, out) = PacketTranslator::translate_connection_outgoing(
            ConnectionState::Login,
            ids::clientbound::login::HELLO.current(),
            &payload,
            JavaMinecraftVersion::V_1_7_6,
        )
        .unwrap();
        assert_eq!(
            id,
            ids::clientbound::login::HELLO.to_id(JavaMinecraftVersion::V_1_7_6)
        );
        // Server id, then the key and token with short lengths
        let mut slice = out.as_slice();
        assert_eq!(&*slice.get_str().unwrap(), "");
        for expected in [&b"public_key_bytes"[..], b"tokn"] {
            let len = slice.get_i16_be().unwrap() as usize;
            assert_eq!(slice.read_slice_borrowed(len).unwrap(), expected);
        }
        assert!(slice.is_empty());
    }

    fn login_success_payload() -> (uuid::Uuid, Vec<u8>) {
        let uuid = uuid::Uuid::from_u128(1);
        let session = uuid::Uuid::from_u128(2);
        let packet = CLoginSuccess::new(&uuid, "Steve", &[], true, session);
        let mut payload = Vec::new();
        packet.write_packet_data(&mut payload).unwrap();
        (uuid, payload)
    }

    #[test]
    fn login_success_outgoing_1_15_writes_uuid_string() {
        let (uuid, payload) = login_success_payload();
        let out = login_success_from_current(&payload, JavaMinecraftVersion::V_1_15).unwrap();
        let mut slice = out.as_slice();
        assert_eq!(slice.get_str_borrowed().unwrap(), uuid.to_string());
        assert_eq!(slice.get_str_borrowed().unwrap(), "Steve");
        assert!(slice.is_empty());
    }

    #[test]
    fn login_success_outgoing_1_21_11_drops_session_id() {
        let (_, payload) = login_success_payload();
        let out = login_success_from_current(&payload, JavaMinecraftVersion::V_1_21_11).unwrap();
        assert_eq!(out, payload[..payload.len() - 16]);
    }
}
