use std::net::IpAddr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpHeaderVersion {
    V4,
    V6,
}

#[derive(Debug, Clone)]
pub struct ParsedPacket {
    pub version: IpHeaderVersion,
    pub src_ip: IpAddr,
    pub dst_ip: IpAddr,
    pub src_port: u16,
    pub dst_port: u16,
    pub tcp_flags: u8,
    pub payload_offset: usize,
}

impl ParsedPacket {
    pub fn is_syn(&self) -> bool {
        (self.tcp_flags & 0x02) != 0 && (self.tcp_flags & 0x10) == 0
    }

    pub fn is_syn_ack(&self) -> bool {
        (self.tcp_flags & 0x12) == 0x12
    }

    pub fn is_fin(&self) -> bool {
        (self.tcp_flags & 0x01) != 0
    }

    pub fn is_rst(&self) -> bool {
        (self.tcp_flags & 0x04) != 0
    }

    pub fn is_ack(&self) -> bool {
        (self.tcp_flags & 0x10) != 0
    }
}

pub fn parse_packet(data: &[u8]) -> Result<ParsedPacket, String> {
    if data.is_empty() {
        return Err("Empty packet".to_string());
    }

    let version = (data[0] >> 4) & 0x0F;
    match version {
        4 => parse_ipv4_packet(data),
        6 => parse_ipv6_packet(data),
        _ => Err(format!("Unknown IP version: {}", version)),
    }
}

fn parse_ipv4_packet(data: &[u8]) -> Result<ParsedPacket, String> {
    if data.len() < 20 {
        return Err("IPv4 packet too short".to_string());
    }

    let ihl = ((data[0] & 0x0F) as usize) * 4;
    if data.len() < ihl + 20 {
        return Err("IPv4 packet too short for TCP".to_string());
    }

    let src_ip = IpAddr::from([data[12], data[13], data[14], data[15]]);
    let dst_ip = IpAddr::from([data[16], data[17], data[18], data[19]]);

    let protocol = data[9];
    if protocol != 6 {
        return Err(format!("Not TCP: protocol={}", protocol));
    }

    parse_tcp_header(data, ihl, src_ip, dst_ip, IpHeaderVersion::V4)
}

fn parse_ipv6_packet(data: &[u8]) -> Result<ParsedPacket, String> {
    if data.len() < 40 + 20 {
        return Err("IPv6 packet too short for TCP".to_string());
    }

    let next_header = data[6];
    if next_header != 6 {
        return Err(format!("Not TCP: next_header={}", next_header));
    }

    let mut src_bytes = [0u8; 16];
    src_bytes.copy_from_slice(&data[8..24]);
    let mut dst_bytes = [0u8; 16];
    dst_bytes.copy_from_slice(&data[24..40]);

    let src_ip = IpAddr::from(src_bytes);
    let dst_ip = IpAddr::from(dst_bytes);

    parse_tcp_header(data, 40, src_ip, dst_ip, IpHeaderVersion::V6)
}

fn parse_tcp_header(
    data: &[u8],
    ip_header_len: usize,
    src_ip: IpAddr,
    dst_ip: IpAddr,
    version: IpHeaderVersion,
) -> Result<ParsedPacket, String> {
    let tcp_offset = ip_header_len;
    if data.len() < tcp_offset + 20 {
        return Err("TCP header too short".to_string());
    }

    let src_port = u16::from_be_bytes([data[tcp_offset], data[tcp_offset + 1]]);
    let dst_port = u16::from_be_bytes([data[tcp_offset + 2], data[tcp_offset + 3]]);
    let tcp_flags = data[tcp_offset + 13];
    let data_offset = ((data[tcp_offset + 12] >> 4) as usize) * 4;
    let payload_offset = tcp_offset + data_offset;

    Ok(ParsedPacket {
        version,
        src_ip,
        dst_ip,
        src_port,
        dst_port,
        tcp_flags,
        payload_offset,
    })
}

pub fn modify_outbound_dnat(
    data: &mut [u8],
    new_dst_ip: IpAddr,
    new_dst_port: u16,
) -> Result<ParsedPacket, String> {
    let version = (data[0] >> 4) & 0x0F;
    match version {
        4 => modify_ipv4_outbound(data, new_dst_ip, new_dst_port),
        6 => modify_ipv6_outbound(data, new_dst_ip, new_dst_port),
        _ => Err(format!("Unknown IP version: {}", version)),
    }
}

fn modify_ipv4_outbound(
    data: &mut [u8],
    new_dst_ip: IpAddr,
    new_dst_port: u16,
) -> Result<ParsedPacket, String> {
    let ihl = ((data[0] & 0x0F) as usize) * 4;

    data[10] = 0;
    data[11] = 0;

    if let IpAddr::V4(ip) = new_dst_ip {
        let octets = ip.octets();
        data[16] = octets[0];
        data[17] = octets[1];
        data[18] = octets[2];
        data[19] = octets[3];
    } else {
        return Err("Cannot map IPv6 address to IPv4 header".to_string());
    }

    let tcp_offset = ihl;
    data[tcp_offset + 2] = (new_dst_port >> 8) as u8;
    data[tcp_offset + 3] = (new_dst_port & 0xFF) as u8;

    data[tcp_offset + 16] = 0;
    data[tcp_offset + 17] = 0;

    recalculate_ipv4_checksum(data);
    recalculate_tcp_checksum_ipv4(data);

    parse_packet(data)
}

fn modify_ipv6_outbound(
    data: &mut [u8],
    new_dst_ip: IpAddr,
    new_dst_port: u16,
) -> Result<ParsedPacket, String> {
    if let IpAddr::V6(ip) = new_dst_ip {
        let octets = ip.octets();
        data[24..40].copy_from_slice(&octets);
    } else {
        return Err("Cannot map IPv4 address to IPv6 header".to_string());
    }

    let tcp_offset = 40;
    data[tcp_offset + 2] = (new_dst_port >> 8) as u8;
    data[tcp_offset + 3] = (new_dst_port & 0xFF) as u8;

    data[tcp_offset + 16] = 0;
    data[tcp_offset + 17] = 0;

    recalculate_tcp_checksum_ipv6(data);

    parse_packet(data)
}

pub fn modify_inbound_snat(
    data: &mut [u8],
    original_src_ip: IpAddr,
    original_src_port: u16,
) -> Result<ParsedPacket, String> {
    let version = (data[0] >> 4) & 0x0F;
    match version {
        4 => modify_ipv4_inbound(data, original_src_ip, original_src_port),
        6 => modify_ipv6_inbound(data, original_src_ip, original_src_port),
        _ => Err(format!("Unknown IP version: {}", version)),
    }
}

fn modify_ipv4_inbound(
    data: &mut [u8],
    original_src_ip: IpAddr,
    original_src_port: u16,
) -> Result<ParsedPacket, String> {
    let ihl = ((data[0] & 0x0F) as usize) * 4;

    data[10] = 0;
    data[11] = 0;

    if let IpAddr::V4(ip) = original_src_ip {
        let octets = ip.octets();
        data[12] = octets[0];
        data[13] = octets[1];
        data[14] = octets[2];
        data[15] = octets[3];
    } else {
        return Err("Cannot map IPv6 address to IPv4 header".to_string());
    }

    let tcp_offset = ihl;
    data[tcp_offset] = (original_src_port >> 8) as u8;
    data[tcp_offset + 1] = (original_src_port & 0xFF) as u8;

    data[tcp_offset + 16] = 0;
    data[tcp_offset + 17] = 0;

    recalculate_ipv4_checksum(data);
    recalculate_tcp_checksum_ipv4(data);

    parse_packet(data)
}

fn modify_ipv6_inbound(
    data: &mut [u8],
    original_src_ip: IpAddr,
    original_src_port: u16,
) -> Result<ParsedPacket, String> {
    if let IpAddr::V6(ip) = original_src_ip {
        let octets = ip.octets();
        data[8..24].copy_from_slice(&octets);
    } else {
        return Err("Cannot map IPv4 address to IPv6 header".to_string());
    }

    let tcp_offset = 40;
    data[tcp_offset] = (original_src_port >> 8) as u8;
    data[tcp_offset + 1] = (original_src_port & 0xFF) as u8;

    data[tcp_offset + 16] = 0;
    data[tcp_offset + 17] = 0;

    recalculate_tcp_checksum_ipv6(data);

    parse_packet(data)
}

fn recalculate_ipv4_checksum(data: &mut [u8]) {
    let ihl = ((data[0] & 0x0F) as usize) * 4;
    let mut sum: u32 = 0;

    for i in (0..ihl).step_by(2) {
        if i + 1 < ihl {
            let word = u16::from_be_bytes([data[i], data[i + 1]]);
            sum += word as u32;
        } else {
            sum += (data[i] as u32) << 8;
        }
    }

    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    let checksum = (!sum as u16).to_be_bytes();
    data[10] = checksum[0];
    data[11] = checksum[1];
}

fn recalculate_tcp_checksum_ipv4(data: &mut [u8]) {
    let ihl = ((data[0] & 0x0F) as usize) * 4;
    let total_len = u16::from_be_bytes([data[2], data[3]]) as usize;
    let tcp_len = total_len.saturating_sub(ihl);

    let src_ip = &data[12..16];
    let dst_ip = &data[16..20];

    let mut sum: u32 = 0;

    sum += u16::from_be_bytes([src_ip[0], src_ip[1]]) as u32;
    sum += u16::from_be_bytes([src_ip[2], src_ip[3]]) as u32;
    sum += u16::from_be_bytes([dst_ip[0], dst_ip[1]]) as u32;
    sum += u16::from_be_bytes([dst_ip[2], dst_ip[3]]) as u32;

    sum += 6u32;
    sum += tcp_len as u32;

    compute_tcp_checksum(data, ihl, tcp_len, &mut sum);
}

fn recalculate_tcp_checksum_ipv6(data: &mut [u8]) {
    let payload_len = u16::from_be_bytes([data[4], data[5]]) as usize;
    let tcp_len = payload_len;

    let src_ip = &data[8..24];
    let dst_ip = &data[24..40];

    let mut sum: u32 = 0;

    for i in (0..16).step_by(2) {
        sum += u16::from_be_bytes([src_ip[i], src_ip[i + 1]]) as u32;
        sum += u16::from_be_bytes([dst_ip[i], dst_ip[i + 1]]) as u32;
    }

    sum += 6u32;
    sum += tcp_len as u32;

    compute_tcp_checksum(data, 40, tcp_len, &mut sum);
}

fn compute_tcp_checksum(data: &mut [u8], tcp_offset: usize, tcp_len: usize, sum: &mut u32) {
    let end = tcp_offset + tcp_len;
    let end_clamped = end.min(data.len());

    let mut i = tcp_offset;
    while i + 1 < end_clamped {
        if i == tcp_offset + 16 || i == tcp_offset + 17 {
            i += 1;
            continue;
        }
        *sum += u16::from_be_bytes([data[i], data[i + 1]]) as u32;
        i += 2;
    }
    if i < end_clamped {
        *sum += (data[i] as u32) << 8;
    }

    while *sum >> 16 != 0 {
        *sum = (*sum & 0xFFFF) + (*sum >> 16);
    }

    let checksum = (!*sum as u16).to_be_bytes();
    data[tcp_offset + 16] = checksum[0];
    data[tcp_offset + 17] = checksum[1];
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    fn build_ipv4_tcp_syn_packet(
        src_ip: Ipv4Addr,
        dst_ip: Ipv4Addr,
        src_port: u16,
        dst_port: u16,
    ) -> Vec<u8> {
        let mut packet = vec![0u8; 54];
        packet[0] = 0x45;
        packet[1] = 0x00;
        let total_len = (packet.len() as u16).to_be_bytes();
        packet[2] = total_len[0];
        packet[3] = total_len[1];
        packet[8] = 64;
        packet[9] = 6;
        let src_octets = src_ip.octets();
        packet[12..16].copy_from_slice(&src_octets);
        let dst_octets = dst_ip.octets();
        packet[16..20].copy_from_slice(&dst_octets);
        let src_port_bytes = src_port.to_be_bytes();
        packet[20] = src_port_bytes[0];
        packet[21] = src_port_bytes[1];
        let dst_port_bytes = dst_port.to_be_bytes();
        packet[22] = dst_port_bytes[0];
        packet[23] = dst_port_bytes[1];
        packet[32] = 0x50;
        packet[33] = 0x02;
        packet
    }

    #[test]
    fn test_parse_ipv4_tcp_packet() {
        let packet = build_ipv4_tcp_syn_packet(
            Ipv4Addr::new(192, 168, 1, 5),
            Ipv4Addr::new(93, 184, 216, 34),
            12345,
            443,
        );
        let parsed = parse_packet(&packet).unwrap();
        assert_eq!(parsed.version, IpHeaderVersion::V4);
        assert_eq!(parsed.src_ip, IpAddr::V4(Ipv4Addr::new(192, 168, 1, 5)));
        assert_eq!(parsed.dst_ip, IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)));
        assert_eq!(parsed.src_port, 12345);
        assert_eq!(parsed.dst_port, 443);
        assert!(parsed.is_syn());
        assert!(!parsed.is_fin());
        assert!(!parsed.is_rst());
    }

    #[test]
    fn test_parse_empty_packet() {
        let result = parse_packet(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_modify_outbound_dnat() {
        let mut packet = build_ipv4_tcp_syn_packet(
            Ipv4Addr::new(192, 168, 1, 5),
            Ipv4Addr::new(93, 184, 216, 34),
            12345,
            443,
        );
        let parsed = modify_outbound_dnat(&mut packet, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 40961).unwrap();
        assert_eq!(parsed.dst_ip, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
        assert_eq!(parsed.dst_port, 40961);
        assert_eq!(parsed.src_ip, IpAddr::V4(Ipv4Addr::new(192, 168, 1, 5)));
        assert_eq!(parsed.src_port, 12345);
    }

    #[test]
    fn test_modify_inbound_snat() {
        let mut packet = build_ipv4_tcp_syn_packet(
            Ipv4Addr::new(127, 0, 0, 1),
            Ipv4Addr::new(192, 168, 1, 5),
            40961,
            12345,
        );
        let parsed = modify_inbound_snat(&mut packet, IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)), 443).unwrap();
        assert_eq!(parsed.src_ip, IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)));
        assert_eq!(parsed.src_port, 443);
        assert_eq!(parsed.dst_ip, IpAddr::V4(Ipv4Addr::new(192, 168, 1, 5)));
        assert_eq!(parsed.dst_port, 12345);
    }

    #[test]
    fn test_tcp_flags() {
        let mut packet = build_ipv4_tcp_syn_packet(
            Ipv4Addr::new(192, 168, 1, 5),
            Ipv4Addr::new(93, 184, 216, 34),
            12345,
            443,
        );
        let parsed = parse_packet(&packet).unwrap();
        assert!(parsed.is_syn());
        assert!(!parsed.is_syn_ack());
        assert!(!parsed.is_ack());

        packet[33] = 0x12;
        let parsed = parse_packet(&packet).unwrap();
        assert!(parsed.is_syn_ack());
        assert!(parsed.is_ack());

        packet[33] = 0x11;
        let parsed = parse_packet(&packet).unwrap();
        assert!(parsed.is_fin());
        assert!(parsed.is_ack());

        packet[33] = 0x14;
        let parsed = parse_packet(&packet).unwrap();
        assert!(parsed.is_rst());
        assert!(parsed.is_ack());
    }

    #[test]
    fn test_ipv4_checksum_calculation() {
        let mut packet = build_ipv4_tcp_syn_packet(
            Ipv4Addr::new(10, 0, 0, 1),
            Ipv4Addr::new(8, 8, 8, 8),
            54321,
            80,
        );
        packet[10] = 0;
        packet[11] = 0;
        recalculate_ipv4_checksum(&mut packet);
        let ip_checksum = u16::from_be_bytes([packet[10], packet[11]]);
        assert_ne!(ip_checksum, 0, "IPv4 checksum should not be zero");

        let mut sum: u32 = 0;
        for i in (0..20).step_by(2) {
            if i == 10 { continue; }
            sum += u16::from_be_bytes([packet[i], packet[i + 1]]) as u32;
        }
        while sum >> 16 != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }
        let expected = !sum as u16;
        assert_eq!(ip_checksum, expected, "IPv4 checksum should match manual calculation");
    }

    #[test]
    fn test_tcp_checksum_ipv4_roundtrip() {
        let mut packet = build_ipv4_tcp_syn_packet(
            Ipv4Addr::new(192, 168, 1, 100),
            Ipv4Addr::new(1, 1, 1, 1),
            9999,
            443,
        );
        recalculate_ipv4_checksum(&mut packet);
        recalculate_tcp_checksum_ipv4(&mut packet);

        let tcp_checksum = u16::from_be_bytes([packet[36], packet[37]]);
        assert_ne!(tcp_checksum, 0, "TCP checksum should not be zero after calculation");

        let original_checksum = tcp_checksum;
        packet[20] ^= 0xFF;
        recalculate_tcp_checksum_ipv4(&mut packet);
        let modified_checksum = u16::from_be_bytes([packet[36], packet[37]]);
        assert_ne!(modified_checksum, original_checksum, "TCP checksum should change when data changes");

        packet[20] ^= 0xFF;
        recalculate_tcp_checksum_ipv4(&mut packet);
        let restored_checksum = u16::from_be_bytes([packet[36], packet[37]]);
        assert_eq!(restored_checksum, original_checksum, "TCP checksum should be deterministic");
    }

    #[test]
    fn test_dnat_snat_checksum_consistency() {
        let original_src = Ipv4Addr::new(192, 168, 1, 5);
        let original_dst = Ipv4Addr::new(93, 184, 216, 34);
        let original_src_port = 12345;
        let original_dst_port = 443;

        let mut packet = build_ipv4_tcp_syn_packet(original_src, original_dst, original_src_port, original_dst_port);
        recalculate_ipv4_checksum(&mut packet);
        recalculate_tcp_checksum_ipv4(&mut packet);

        let proxy_ip = Ipv4Addr::new(127, 0, 0, 1);
        let proxy_port = 40961u16;

        let parsed_after_dnat = modify_outbound_dnat(&mut packet, IpAddr::V4(proxy_ip), proxy_port).unwrap();
        assert_eq!(parsed_after_dnat.dst_ip, IpAddr::V4(proxy_ip));
        assert_eq!(parsed_after_dnat.dst_port, proxy_port);

        let dnat_ip_checksum = u16::from_be_bytes([packet[10], packet[11]]);
        let dnat_tcp_checksum = u16::from_be_bytes([packet[36], packet[37]]);
        assert_ne!(dnat_ip_checksum, 0);
        assert_ne!(dnat_tcp_checksum, 0);

        let parsed_after_snat = modify_inbound_snat(&mut packet, IpAddr::V4(original_dst), original_dst_port).unwrap();
        assert_eq!(parsed_after_snat.src_ip, IpAddr::V4(original_dst));
        assert_eq!(parsed_after_snat.src_port, original_dst_port);
    }
}
