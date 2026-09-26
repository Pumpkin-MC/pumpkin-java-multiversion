//! Network NBT: copied as bytes, with a named root before 1.20.2.

use pumpkin_util::version::JavaMinecraftVersion;

eras! {
    pub enum NbtRoot {
        /// The root compound has an (empty) name.
        Named = V_1_7_2,
        /// Nameless root.
        Nameless = V_1_20_2,
    }
}

fn be_len(data: &[u8], at: usize, width: usize) -> Option<usize> {
    let bytes = data.get(at..at + width)?;
    let value = bytes.iter().fold(0i64, |acc, &b| (acc << 8) | i64::from(b));
    // i32 lengths are signed
    let value = if width == 4 {
        i64::from(value as u32 as i32)
    } else {
        value
    };
    usize::try_from(value).ok()
}

/// Byte length of one `tag_type` payload at the start of `data`.
fn payload_len(data: &[u8], tag_type: u8) -> Option<usize> {
    Some(match tag_type {
        0 => 0,
        1 => 1,
        2 => 2,
        3 | 5 => 4,
        4 | 6 => 8,
        7 => 4 + be_len(data, 0, 4)?,
        8 => 2 + be_len(data, 0, 2)?,
        9 => {
            let element = *data.first()?;
            let count = be_len(data, 1, 4)?;
            let mut at = 5;
            for _ in 0..count {
                at += payload_len(data.get(at..)?, element)?;
            }
            at
        }
        10 => {
            let mut at = 0;
            loop {
                let child = *data.get(at)?;
                at += 1;
                if child == 0 {
                    break at;
                }
                at += 2 + be_len(data, at, 2)?;
                at += payload_len(data.get(at..)?, child)?;
            }
        }
        11 => 4 + 4 * be_len(data, 0, 4)?,
        12 => 4 + 8 * be_len(data, 0, 4)?,
        _ => return None,
    })
}

/// Splits one 26.3 network NBT (type byte + nameless root) off `data`.
pub fn split_network_nbt<'a>(data: &mut &'a [u8]) -> Option<&'a [u8]> {
    let tag_type = *data.first()?;
    let len = 1 + payload_len(data.get(1..)?, tag_type)?;
    let (nbt, rest) = data.split_at_checked(len)?;
    *data = rest;
    Some(nbt)
}

/// Writes a 26.3 network NBT for `version`.
pub fn write_network_nbt(nbt: &[u8], version: JavaMinecraftVersion, out: &mut Vec<u8>) {
    let Some((&tag_type, payload)) = nbt.split_first() else {
        return;
    };
    out.push(tag_type);
    if tag_type != 0 && NbtRoot::of(version) == NbtRoot::Named {
        out.extend_from_slice(&[0, 0]);
    }
    out.extend_from_slice(payload);
}

#[cfg(test)]
mod tests {
    use pumpkin_nbt::{Nbt, compound::NbtCompound, tag::NbtTag};

    use super::*;

    #[test]
    fn split_matches_written_length() {
        let mut inner = NbtCompound::new();
        inner.put("list", NbtTag::List(vec![NbtTag::Int(1), NbtTag::Int(2)]));
        inner.put("longs", NbtTag::LongArray(vec![1, 2, 3]));
        let mut root = NbtCompound::new();
        root.put("name", NbtTag::String("chest".into()));
        root.put("inner", NbtTag::Compound(inner));
        let bytes = Nbt::from(root).write_unnamed();

        let mut data = [bytes.as_ref(), &[0xAB]].concat();
        let mut slice = data.as_mut_slice() as &[u8];
        let nbt = split_network_nbt(&mut slice).unwrap();
        assert_eq!(nbt, bytes.as_ref());
        assert_eq!(slice, &[0xAB]);
    }
}
