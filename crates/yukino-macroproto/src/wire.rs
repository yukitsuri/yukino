use std::io::{self, Error, ErrorKind, Read, Write};

use crate::model::{MacroRequest, MacroResponse};

pub const MAX_PAYLOAD_BYTES: u32 = 64 * 1024 * 1024;

pub fn write_request<W: Write>(writer: &mut W, req: &MacroRequest) -> io::Result<()> {
    let payload = serde_json::to_vec(req)
        .map_err(|e| Error::new(ErrorKind::InvalidData, format!("failed to serialize request: {e}")))?;
    write_frame(writer, &payload)
}

pub fn read_request<R: Read>(reader: &mut R) -> io::Result<MacroRequest> {
    let payload = read_frame(reader)?;
    serde_json::from_slice(&payload)
        .map_err(|e| Error::new(ErrorKind::InvalidData, format!("failed to deserialize request: {e}")))
}

pub fn write_response<W: Write>(writer: &mut W, res: &MacroResponse) -> io::Result<()> {
    let payload = serde_json::to_vec(res)
        .map_err(|e| Error::new(ErrorKind::InvalidData, format!("failed to serialize response: {e}")))?;
    write_frame(writer, &payload)
}

pub fn read_response<R: Read>(reader: &mut R) -> io::Result<MacroResponse> {
    let payload = read_frame(reader)?;
    serde_json::from_slice(&payload)
        .map_err(|e| Error::new(ErrorKind::InvalidData, format!("failed to deserialize response: {e}")))
}

fn write_frame<W: Write>(writer: &mut W, payload: &[u8]) -> io::Result<()> {
    let len = u32::try_from(payload.len())
        .map_err(|_| Error::new(ErrorKind::InvalidInput, "payload too large for u32 length prefix"))?;
    if len > MAX_PAYLOAD_BYTES {
        return Err(Error::new(ErrorKind::InvalidInput, "payload exceeds MAX_PAYLOAD_BYTES limit"));
    }
    writer.write_all(&len.to_le_bytes())?;
    writer.write_all(payload)?;
    writer.flush()
}

fn read_frame<R: Read>(reader: &mut R) -> io::Result<Vec<u8>> {
    let mut len_bytes = [0u8; 4];
    reader.read_exact(&mut len_bytes)?;
    let len = u32::from_le_bytes(len_bytes);
    if len > MAX_PAYLOAD_BYTES {
        return Err(Error::new(ErrorKind::InvalidData, "incoming frame exceeds MAX_PAYLOAD_BYTES"));
    }
    let mut payload = vec![0u8; len as usize];
    reader.read_exact(&mut payload)?;
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{MacroDiagnostic, MacroKind, MacroSpan};
    use std::io::Cursor;

    #[test]
    fn test_request_roundtrip() {
        let req = MacroRequest::new(
            "json_model",
            MacroKind::Mutation,
            "struct User { name: string }",
            Some("rename_all=\"snake\"".to_string()),
            "x86_64-unknown-linux-gnu",
            "debug",
            Some("/path/to/project".to_string()),
        );

        let mut buf = Vec::new();
        write_request(&mut buf, &req).expect("write request");

        let mut cursor = Cursor::new(buf);
        let decoded = read_request(&mut cursor).expect("read request");
        assert_eq!(req, decoded);
    }

    #[test]
    fn test_response_roundtrip_with_diagnostics() {
        let mut res = MacroResponse::success(
            "fun user_to_json() { space }",
            vec!["assets/schema.json".to_string()],
        );
        res.diagnostics.push(MacroDiagnostic::warning(
            "field unused in serialization",
            Some(MacroSpan::new(1, 10, 25, 2, 5)),
        ));

        let mut buf = Vec::new();
        write_response(&mut buf, &res).expect("write response");

        let mut cursor = Cursor::new(buf);
        let decoded = read_response(&mut cursor).expect("read response");
        assert_eq!(res, decoded);
    }

    #[test]
    fn test_read_frame_too_large() {
        let huge_len: u32 = MAX_PAYLOAD_BYTES + 1;
        let mut buf = huge_len.to_le_bytes().to_vec();
        buf.extend_from_slice(&[0u8; 10]);

        let mut cursor = Cursor::new(buf);
        let res = read_frame(&mut cursor);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().kind(), ErrorKind::InvalidData);
    }
}
