pub fn extract_sni_from_client_hello(data: &[u8]) -> Option<String> {
    if data.len() < 5 {
        return None;
    }

    if data[0] != 0x16 {
        return None;
    }

    let tls_version = u16::from_be_bytes([data[1], data[2]]);
    if tls_version < 0x0301 {
        return None;
    }

    let record_len = u16::from_be_bytes([data[3], data[4]]) as usize;
    if data.len() < 5 + record_len {
        return None;
    }

    let handshake = &data[5..5 + record_len];

    if handshake.is_empty() || handshake[0] != 0x01 {
        return None;
    }

    if handshake.len() < 4 {
        return None;
    }

    let handshake_len = ((handshake[1] as usize) << 16)
        | ((handshake[2] as usize) << 8)
        | (handshake[3] as usize);
    if handshake.len() < 4 + handshake_len {
        return None;
    }

    let client_hello = &handshake[4..4 + handshake_len];

    if client_hello.len() < 34 {
        return None;
    }

    let _ch_version = u16::from_be_bytes([client_hello[0], client_hello[1]]);
    let _random = &client_hello[2..34];

    let mut offset = 34;

    if offset >= client_hello.len() {
        return None;
    }
    let session_id_len = client_hello[offset] as usize;
    offset += 1 + session_id_len;

    if offset + 2 > client_hello.len() {
        return None;
    }
    let cipher_suites_len = u16::from_be_bytes([client_hello[offset], client_hello[offset + 1]]) as usize;
    offset += 2 + cipher_suites_len;

    if offset >= client_hello.len() {
        return None;
    }
    let compression_methods_len = client_hello[offset] as usize;
    offset += 1 + compression_methods_len;

    if offset + 2 > client_hello.len() {
        return None;
    }
    let extensions_len = u16::from_be_bytes([client_hello[offset], client_hello[offset + 1]]) as usize;
    offset += 2;

    let extensions_end = offset + extensions_len;
    if extensions_end > client_hello.len() {
        return None;
    }

    let extensions = &client_hello[offset..extensions_end];
    let mut ext_offset = 0;

    while ext_offset + 4 <= extensions.len() {
        let ext_type = u16::from_be_bytes([extensions[ext_offset], extensions[ext_offset + 1]]);
        let ext_len = u16::from_be_bytes([extensions[ext_offset + 2], extensions[ext_offset + 3]]) as usize;
        ext_offset += 4;

        if ext_offset + ext_len > extensions.len() {
            break;
        }

        if ext_type == 0x0000 {
            return parse_sni_extension(&extensions[ext_offset..ext_offset + ext_len]);
        }

        ext_offset += ext_len;
    }

    None
}

fn parse_sni_extension(data: &[u8]) -> Option<String> {
    if data.len() < 2 {
        return None;
    }

    let _list_len = u16::from_be_bytes([data[0], data[1]]) as usize;
    let mut offset = 2;

    while offset + 3 <= data.len() {
        let name_type = data[offset];
        let name_len = u16::from_be_bytes([data[offset + 1], data[offset + 2]]) as usize;
        offset += 3;

        if offset + name_len > data.len() {
            break;
        }

        if name_type == 0x00 {
            let name_bytes = &data[offset..offset + name_len];
            return std::str::from_utf8(name_bytes).map(|s| s.to_string()).ok();
        }

        offset += name_len;
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_client_hello_with_sni(hostname: &str) -> Vec<u8> {
        let mut ch = Vec::new();

        ch.extend_from_slice(&[0x03, 0x03]);
        ch.extend_from_slice(&[0u8; 32]);

        ch.push(0x00);

        ch.extend_from_slice(&[0x00, 0x02]);
        ch.extend_from_slice(&[0x00, 0x2f]);

        ch.push(0x01);
        ch.push(0x00);

        let mut sni_ext = Vec::new();
        let hostname_bytes = hostname.as_bytes();
        let mut sni_list = Vec::new();
        sni_list.push(0x00);
        sni_list.extend_from_slice(&(hostname_bytes.len() as u16).to_be_bytes());
        sni_list.extend_from_slice(hostname_bytes);
        sni_ext.extend_from_slice(&(sni_list.len() as u16).to_be_bytes());
        sni_ext.extend_from_slice(&sni_list);

        let mut extensions = Vec::new();
        extensions.extend_from_slice(&[0x00, 0x00]);
        extensions.extend_from_slice(&(sni_ext.len() as u16).to_be_bytes());
        extensions.extend_from_slice(&sni_ext);

        ch.extend_from_slice(&(extensions.len() as u16).to_be_bytes());
        ch.extend_from_slice(&extensions);

        let mut handshake = Vec::new();
        handshake.push(0x01);
        let ch_len = ch.len();
        handshake.push(((ch_len >> 16) & 0xFF) as u8);
        handshake.push(((ch_len >> 8) & 0xFF) as u8);
        handshake.push((ch_len & 0xFF) as u8);
        handshake.extend_from_slice(&ch);

        let mut record = Vec::new();
        record.push(0x16);
        record.extend_from_slice(&[0x03, 0x01]);
        let hs_len = handshake.len();
        record.extend_from_slice(&(hs_len as u16).to_be_bytes());
        record.extend_from_slice(&handshake);

        record
    }

    #[test]
    fn test_extract_sni_basic() {
        let data = build_client_hello_with_sni("example.com");
        let sni = extract_sni_from_client_hello(&data);
        assert_eq!(sni, Some("example.com".to_string()));
    }

    #[test]
    fn test_extract_sni_wildcard() {
        let data = build_client_hello_with_sni("*.example.com");
        let sni = extract_sni_from_client_hello(&data);
        assert_eq!(sni, Some("*.example.com".to_string()));
    }

    #[test]
    fn test_extract_sni_empty_data() {
        assert_eq!(extract_sni_from_client_hello(&[]), None);
        assert_eq!(extract_sni_from_client_hello(&[0x16]), None);
    }

    #[test]
    fn test_extract_sni_not_tls() {
        let data = vec![0x17, 0x03, 0x01, 0x00, 0x05, 0x01, 0x00, 0x00, 0x00, 0x00];
        assert_eq!(extract_sni_from_client_hello(&data), None);
    }

    #[test]
    fn test_extract_sni_truncated() {
        let mut data = build_client_hello_with_sni("example.com");
        data.truncate(10);
        assert_eq!(extract_sni_from_client_hello(&data), None);
    }

    #[test]
    fn test_extract_sni_malformed() {
        let data = vec![0x16, 0x03, 0x01, 0x00, 0x02, 0xFF, 0xFF];
        assert_eq!(extract_sni_from_client_hello(&data), None);
    }
}
