pub mod nat_table;
pub mod packet_processor;
pub mod sni_parser;
pub mod elevation;
pub mod wifi_detect;

#[cfg(feature = "windivert")]
pub mod diverter;

pub use nat_table::{NatTable, NatEntry, ConnKey, OriginalDest, NatEntryState};
pub use packet_processor::{ParsedPacket, IpHeaderVersion, parse_packet, modify_outbound_dnat, modify_inbound_snat};
#[cfg(feature = "windivert")]
pub use diverter::{PacketDiverter, DivertConfig, DivertError};
pub use sni_parser::extract_sni_from_client_hello;
pub use elevation::{is_elevated, request_elevation, ensure_elevated};
pub use wifi_detect::{is_wifi_adapter, detect_primary_adapter, AdapterInfo};
