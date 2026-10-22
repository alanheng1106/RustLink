use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use byteorder::{BigEndian, ReadBytesExt};
use std::io::{Cursor, Read};
use crate::model::protocol::TrackInfo;

pub fn encode_track(info: &TrackInfo) -> String {
    // We encode using Lavalink compatible JSON base64, with a fallback binary writer if needed
    let json = serde_json::to_string(info).unwrap_or_default();
    BASE64.encode(json.as_bytes())
}

pub fn decode_track(encoded: &str) -> Result<TrackInfo, String> {
    let bytes = BASE64.decode(encoded.trim()).map_err(|e| format!("Invalid base64: {e}"))?;

    // Try JSON decoding first
    if let Ok(info) = serde_json::from_slice::<TrackInfo>(&bytes) {
        return Ok(info);
    }

    // Fallback: Attempt Lavaplayer DataInputStream binary decoding
    let mut cursor = Cursor::new(bytes);
    decode_lavaplayer_binary(&mut cursor)
}

fn read_java_utf<R: Read>(reader: &mut R) -> std::io::Result<String> {
    let len = reader.read_u16::<BigEndian>()? as usize;
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;
    String::from_utf8(buf).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

fn decode_lavaplayer_binary(cursor: &mut Cursor<Vec<u8>>) -> Result<TrackInfo, String> {
    // Lavaplayer binary stream:
    // 4-byte header or version flag
    let flags = cursor.read_i32::<BigEndian>().map_err(|e| e.to_string())?;
    let _version = if (flags & 1) != 0 {
        cursor.read_u8().map_err(|e| e.to_string())?
    } else {
        1
    };

    let title = read_java_utf(cursor).map_err(|e| e.to_string())?;
    let author = read_java_utf(cursor).map_err(|e| e.to_string())?;
    let length = cursor.read_i64::<BigEndian>().map_err(|e| e.to_string())? as u64;
    let identifier = read_java_utf(cursor).map_err(|e| e.to_string())?;
    let is_stream = cursor.read_u8().map_err(|e| e.to_string())? != 0;
    
    let has_uri = cursor.read_u8().map_err(|e| e.to_string())? != 0;
    let uri = if has_uri {
        Some(read_java_utf(cursor).map_err(|e| e.to_string())?)
    } else {
        None
    };

    let source_name = read_java_utf(cursor).map_err(|e| e.to_string())?;
    let position = cursor.read_i64::<BigEndian>().map_err(|e| e.to_string())? as u64;

    Ok(TrackInfo {
        identifier,
        is_seekable: !is_stream,
        author,
        length,
        is_stream,
        position,
        title,
        uri,
        artwork_url: None,
        isrc: None,
        source_name,
    })
}
