//! DDC/CI Capabilities String Parser
//!
//! Implements parsing of the VESA MCCS capabilities string (VCP code 0xF3).
//! The capabilities string is a structured response that describes which VCP
//! features a monitor supports, along with protocol versions and model info.
//!
//! # Capabilities String Format
//!
//! The format follows VESA Monitor Control Command Set (MCCS) specification:
//!
//! ```text
//! protver(2.1) mccs_ver(2.1) model("DELL S2721QS") vcp(10 12 60(03 0b) D6 ...)
//! ```
//!
//! - `protver(x.y)` — Protocol version
//! - `mccs_ver(x.y)` — MCCS version
//! - `model("name")` — Monitor model name
//! - `vcp(...)` — List of supported VCP codes with optional values
//! - `cmds(...)` — List of supported commands

use std::collections::HashSet;

/// Parsed DDC/CI capabilities string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilitiesString {
    /// Protocol version (e.g., "2.1").
    pub protocol_version: String,
    /// MCCS version (e.g., "2.1").
    pub mccs_version: Option<String>,
    /// Monitor model name, if present.
    pub model_name: Option<String>,
    /// Set of supported VCP code values.
    pub supported_vcp_codes: HashSet<u8>,
    /// Set of supported command codes.
    pub supported_commands: HashSet<u8>,
    /// Raw capabilities string as received from the monitor.
    pub raw: String,
}

impl CapabilitiesString {
    /// Returns true if the monitor supports the given VCP code.
    pub fn supports_vcp_code(&self, code: u8) -> bool {
        self.supported_vcp_codes.contains(&code)
    }

    /// Returns the number of supported VCP features.
    pub fn vcp_feature_count(&self) -> usize {
        self.supported_vcp_codes.len()
    }

    /// Returns true if brightness (0x10) is supported.
    pub fn supports_brightness(&self) -> bool {
        self.supports_vcp_code(0x10)
    }

    /// Returns true if contrast (0x12) is supported.
    pub fn supports_contrast(&self) -> bool {
        self.supports_vcp_code(0x12)
    }

    /// Returns true if input source (0x60) is supported.
    pub fn supports_input_source(&self) -> bool {
        self.supports_vcp_code(0x60)
    }

    /// Returns true if power control (0xD6) is supported.
    pub fn supports_power_control(&self) -> bool {
        self.supports_vcp_code(0xD6)
    }
}

/// Error type for capabilities string parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilitiesParseError {
    /// The capabilities string is empty or whitespace-only.
    Empty,
    /// A required token (like `protver`) is missing.
    MissingToken(String),
    /// A token could not be parsed (malformed syntax).
    MalformedToken {
        /// The name of the malformed token.
        token: String,
        /// A human-readable description of what went wrong.
        details: String,
    },
    /// A hex value in a `vcp(...)` block could not be parsed.
    InvalidHexValue(String),
}

impl std::fmt::Display for CapabilitiesParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "capabilities string is empty"),
            Self::MissingToken(t) => write!(f, "missing required token: {}", t),
            Self::MalformedToken { token, details } => {
                write!(f, "malformed token '{}': {}", token, details)
            }
            Self::InvalidHexValue(v) => write!(f, "invalid hex value: {}", v),
        }
    }
}

/// Extracts the content inside the first `(` `)` group after a keyword.
/// Returns None if no matching group is found.
fn extract_group(input: &str, keyword: &str) -> Option<String> {
    let keyword_start = input.find(keyword)?;
    let after_keyword = &input[keyword_start + keyword.len()..];

    // Find the first opening paren
    let paren_start = after_keyword.find('(')?;
    let rest = &after_keyword[paren_start + 1..];

    // Find matching closing paren, respecting nested parens
    let mut depth = 1;
    let mut content = String::new();
    for ch in rest.chars() {
        match ch {
            '(' => {
                depth += 1;
                content.push(ch);
            }
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(content);
                }
                content.push(ch);
            }
            _ => {
                content.push(ch);
            }
        }
    }
    None // No matching closing paren found
}

/// Parses hex values from the content of a `vcp(...)` or `cmds(...)` group.
/// Properly handles nested groups like `60(03 0b)` - extracts only the outer
/// hex values (the VCP codes themselves), not the sub-values.
fn parse_hex_values(content: &str) -> Vec<u16> {
    let mut values = Vec::new();
    let mut current = String::new();
    let mut depth = 0;

    for ch in content.chars() {
        match ch {
            '(' => {
                // Flush current token as a VCP code before entering sub-values
                if !current.is_empty() && depth == 0 {
                    if let Ok(val) = u16::from_str_radix(&current, 16) {
                        values.push(val);
                    }
                    current.clear();
                }
                depth += 1;
            }
            ')' => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            _ if ch.is_whitespace() => {
                if depth == 0 && !current.is_empty() {
                    if let Ok(val) = u16::from_str_radix(&current, 16) {
                        values.push(val);
                    }
                    current.clear();
                }
            }
            _ => {
                if depth == 0 {
                    current.push(ch);
                }
            }
        }
    }
    // Flush remaining token
    if !current.is_empty() {
        if let Ok(val) = u16::from_str_radix(&current, 16) {
            values.push(val);
        }
    }

    values
}

/// Extracts a quoted string content, e.g., `"DELL S2721QS"` => `DELL S2721QS`
fn extract_quoted(content: &str) -> &str {
    let trimmed = content.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    }
}

/// Parses a DDC/CI capabilities string into a structured [`CapabilitiesString`].
///
/// # Errors
///
/// Returns [`CapabilitiesParseError::Empty`] if the input is empty.
/// Returns [`CapabilitiesParseError::MissingToken`] if `protver` is missing.
pub fn parse_capabilities_string(
    input: &str,
) -> Result<CapabilitiesString, CapabilitiesParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(CapabilitiesParseError::Empty);
    }

    let mut caps = CapabilitiesString {
        protocol_version: String::new(),
        mccs_version: None,
        model_name: None,
        supported_vcp_codes: HashSet::new(),
        supported_commands: HashSet::new(),
        raw: trimmed.to_string(),
    };

    // Extract protocol version
    if let Some(content) = extract_group(trimmed, "protver") {
        caps.protocol_version = content.trim().to_string();
    } else {
        return Err(CapabilitiesParseError::MissingToken("protver".into()));
    }

    // Extract MCCS version (optional)
    if let Some(content) = extract_group(trimmed, "mccs_ver") {
        caps.mccs_version = Some(content.trim().to_string());
    }

    // Extract model name (optional)
    if let Some(content) = extract_group(trimmed, "model") {
        let name = extract_quoted(&content).trim().to_string();
        if !name.is_empty() {
            caps.model_name = Some(name);
        }
    }

    // Extract VCP codes
    if let Some(content) = extract_group(trimmed, "vcp") {
        let codes = parse_hex_values(&content);
        for code in codes {
            if code <= 0xFF {
                caps.supported_vcp_codes.insert(code as u8);
            }
        }
    }

    // Extract commands
    if let Some(content) = extract_group(trimmed, "cmds") {
        let codes = parse_hex_values(&content);
        for code in codes {
            if code <= 0xFF {
                caps.supported_commands.insert(code as u8);
            }
        }
    }

    Ok(caps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        let result = parse_capabilities_string("");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CapabilitiesParseError::Empty);
    }

    #[test]
    fn test_parse_whitespace_only() {
        let result = parse_capabilities_string("   ");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CapabilitiesParseError::Empty);
    }

    #[test]
    fn test_parse_minimal() {
        let caps = parse_capabilities_string("protver(2.0)").unwrap();
        assert_eq!(caps.protocol_version, "2.0");
        assert_eq!(caps.mccs_version, None);
        assert_eq!(caps.model_name, None);
        assert!(caps.supported_vcp_codes.is_empty());
    }

    #[test]
    fn test_parse_full_capabilities() {
        let input =
            r#"protver(2.1) mccs_ver(2.1) model("DELL S2721QS") vcp(10 12 60(03 0b) D6 AC 16)"#;
        let caps = parse_capabilities_string(input).unwrap();

        assert_eq!(caps.protocol_version, "2.1");
        assert_eq!(caps.mccs_version, Some("2.1".to_string()));
        assert_eq!(caps.model_name, Some("DELL S2721QS".to_string()));
        assert!(caps.supports_vcp_code(0x10)); // brightness
        assert!(caps.supports_vcp_code(0x12)); // contrast
        assert!(caps.supports_vcp_code(0x60)); // input source
        assert!(caps.supports_vcp_code(0xD6)); // power control
        assert!(caps.supports_vcp_code(0xAC)); // audio volume
        assert!(caps.supports_vcp_code(0x16)); // video gain
        assert_eq!(caps.vcp_feature_count(), 6);
    }

    #[test]
    fn test_parse_missing_protver() {
        let result = parse_capabilities_string("mccs_ver(2.1) vcp(10 12)");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            CapabilitiesParseError::MissingToken("protver".into())
        );
    }

    #[test]
    fn test_parse_supports_brightness() {
        let caps = parse_capabilities_string("protver(2.0) vcp(10 12 60)").unwrap();
        assert!(caps.supports_brightness());
        assert!(caps.supports_contrast());
        assert!(caps.supports_input_source());
        assert!(!caps.supports_power_control());
    }

    #[test]
    fn test_parse_vcp_with_sub_values() {
        let input = r#"protver(2.1) mccs_ver(2.1) model("DELL S2721QS") vcp(10 12 60(03 0b) D6)"#;
        let caps = parse_capabilities_string(input).unwrap();

        // The sub-values (03 0b) for 0x60 should NOT be parsed as separate VCP codes
        assert!(caps.supports_vcp_code(0x60));
        assert!(!caps.supports_vcp_code(0x03));
        assert!(!caps.supports_vcp_code(0x0b));
    }

    #[test]
    fn test_parse_commands() {
        let input = "protver(2.1) cmds(01 02 03)";
        let caps = parse_capabilities_string(input).unwrap();
        assert!(caps.supported_commands.contains(&0x01));
        assert!(caps.supported_commands.contains(&0x02));
        assert!(caps.supported_commands.contains(&0x03));
        assert_eq!(caps.supported_commands.len(), 3);
    }

    #[test]
    fn test_parse_value_range() {
        let input = "protver(2.0) vcp(10 12 60 00-FF)";
        let caps = parse_capabilities_string(input).unwrap();
        assert!(caps.supports_vcp_code(0x10));
        assert!(caps.supports_vcp_code(0x60));
        // 00-FF should NOT be parsed - we don't support ranges currently
        assert_eq!(caps.supported_vcp_codes.len(), 3);
    }

    #[test]
    fn test_real_world_capabilities_benq() {
        let input = r#"protver(2.1) mccs_ver(2.1) model("BenQ GW2765") vcp(10 12 16 18 1A 50 5B 5C 60(01 03 04 0B 0C 11 12) 62 AC AE B2 B6 C0 C6 C8 C9 D6 DF E0 E1 E2 F0 F5 FF) cmds(01 02 03 07 0C E3 F3) vcp_p02(10 12 16 18 1A 50 5B 5C 60 62 AC AE B2 B6 C0 C6 C8 C9 D6 DF E0 E1 E2 F0 F5 FF)"#;
        let caps = parse_capabilities_string(input).unwrap();

        assert_eq!(caps.protocol_version, "2.1");
        assert_eq!(caps.model_name, Some("BenQ GW2765".to_string()));
        assert!(caps.supports_brightness());
        assert!(caps.supports_contrast());
        assert!(caps.supports_input_source());
        assert!(caps.supports_power_control());
        assert!(caps.supports_vcp_code(0x62)); // audio volume
        assert!(caps.supports_vcp_code(0xAC)); // audio treble
        assert!(caps.supports_vcp_code(0xB6)); // display mode
        assert!(caps.supports_vcp_code(0xDF)); // VCP version
        assert!(caps.supported_commands.contains(&0xF3)); // capabilities query
    }

    #[test]
    fn test_extract_group_simple() {
        let input = "protver(2.0) vcp(10 12)";
        assert_eq!(extract_group(input, "protver"), Some("2.0".to_string()));
        assert_eq!(extract_group(input, "vcp"), Some("10 12".to_string()));
    }

    #[test]
    fn test_extract_group_nested() {
        let input = "vcp(60(03 0b) D6)";
        let content = extract_group(input, "vcp").unwrap();
        assert_eq!(content, "60(03 0b) D6");
    }

    #[test]
    fn test_parse_hex_values_simple() {
        let values = parse_hex_values("10 12 60 D6");
        assert_eq!(values, vec![0x10, 0x12, 0x60, 0xD6]);
    }

    #[test]
    fn test_parse_hex_values_with_subvalues() {
        let values = parse_hex_values("60(03 0b) D6 AC");
        // Should extract only the outer VCP codes, not sub-values
        assert_eq!(values, vec![0x60, 0xD6, 0xAC]);
    }

    #[test]
    fn test_parse_hex_values_empty() {
        let values = parse_hex_values("");
        assert!(values.is_empty());
    }

    #[test]
    fn test_capabilities_display() {
        let err = CapabilitiesParseError::InvalidHexValue("XYZ".to_string());
        assert_eq!(format!("{}", err), "invalid hex value: XYZ");

        let err = CapabilitiesParseError::MissingToken("protver".to_string());
        assert_eq!(format!("{}", err), "missing required token: protver");
    }
}
