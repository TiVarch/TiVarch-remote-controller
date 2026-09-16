use anyhow::Result;
use qrcode::render::svg;
use qrcode::render::unicode;
use qrcode::QrCode;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::time::{Duration, Instant};
use tracing::info;
use uuid::Uuid;

const PAIRING_TOKEN_TTL: Duration = Duration::from_secs(600); // 10 minutes

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QrPayload {
    pub v: u8,
    pub name: String,
    pub ip: String,
    pub port: u16,
    pub token: String,
    pub ble_mac: Option<String>,
}

#[derive(Clone)]
pub struct PairingSession {
    pub active_token: String,
    pub payload: QrPayload,
    pub created_at: Instant,
    pub is_used: bool,
}

impl PairingSession {
    pub fn new(server_name: &str, port: u16, ble_mac: Option<String>) -> Result<Self> {
        let best_ip = Self::detect_primary_ip();
        info!("Selected network IP for pairing: {}", best_ip);
        Self::with_ip(server_name, port, best_ip, ble_mac)
    }

    pub fn with_ip(
        server_name: &str,
        port: u16,
        ip: String,
        ble_mac: Option<String>,
    ) -> Result<Self> {
        let token = Uuid::new_v4().to_string();

        let payload = QrPayload {
            v: 1,
            name: server_name.to_string(),
            ip,
            port,
            token: token.clone(),
            ble_mac,
        };

        Ok(Self {
            active_token: token,
            payload,
            created_at: Instant::now(),
            is_used: false,
        })
    }

    /// Selects genuine physical wlan/eth IP and ignores virtual adapters (Docker, Tailscale, etc.)
    pub fn detect_primary_ip() -> String {
        if let Ok(interfaces) = local_ip_address::list_afinet_netifas() {
            let mut candidates = Vec::new();

            for (name, ip) in interfaces {
                if let IpAddr::V4(ipv4) = ip {
                    if ipv4.is_loopback() || ipv4.is_link_local() {
                        continue;
                    }

                    let if_name = name.to_lowercase();
                    // Exclude virtual/bridge/vpn interfaces
                    if if_name.starts_with("docker")
                        || if_name.starts_with("veth")
                        || if_name.starts_with("tun")
                        || if_name.starts_with("tap")
                        || if_name.starts_with("virbr")
                        || if_name.starts_with("br-")
                        || if_name.contains("tailscale")
                        || if_name.contains("wg")
                    {
                        continue;
                    }

                    // Score candidate interface
                    let priority = if if_name.starts_with("wl") {
                        3 // Wi-Fi preferred for TV / Remote context
                    } else if if_name.starts_with("eth") || if_name.starts_with("en") {
                        2 // Wired Ethernet
                    } else {
                        1
                    };

                    candidates.push((priority, ipv4.to_string()));
                }
            }

            candidates.sort_by(|a, b| b.0.cmp(&a.0));
            if let Some((_, ip_str)) = candidates.first() {
                return ip_str.clone();
            }
        }

        // Fallback
        local_ip_address::local_ip()
            .map(|ip| ip.to_string())
            .unwrap_or_else(|_| "127.0.0.1".to_string())
    }

    pub fn is_expired(&self) -> bool {
        self.is_used || self.created_at.elapsed() > PAIRING_TOKEN_TTL
    }

    pub fn invalidate(&mut self) {
        self.is_used = true;
    }

    pub fn render_terminal_qr(&self) -> Result<String> {
        let json_data = serde_json::to_string(&self.payload)?;
        let code = QrCode::new(json_data.as_bytes())?;

        let rendered = code
            .render::<unicode::Dense1x2>()
            .dark_color(unicode::Dense1x2::Light)
            .light_color(unicode::Dense1x2::Dark)
            .build();

        Ok(rendered)
    }

    pub fn render_svg(&self) -> Result<String> {
        let json_data = serde_json::to_string(&self.payload)?;
        let code = QrCode::new(json_data.as_bytes())?;

        let svg_image = code
            .render()
            .min_dimensions(300, 300)
            .dark_color(svg::Color("#F0F6FC"))
            .light_color(svg::Color("#0D1117"))
            .build();

        Ok(svg_image)
    }

    pub fn print_terminal_qr(&self) -> Result<()> {
        let qr_graphic = self.render_terminal_qr()?;

        println!("\n========================================================");
        println!("          TIVARCH ULTRA PAIRING QR CODE                 ");
        println!("========================================================");
        println!("{}", qr_graphic);
        println!("Server Name  : {}", self.payload.name);
        println!("IP Address   : {}", self.payload.ip);
        println!("Port         : {}", self.payload.port);
        println!("BLE Address  : {:?}", self.payload.ble_mac);
        println!("Pairing Token: {}", self.active_token);
        println!("Valid For    : 10 Minutes (Single Use)");
        println!("========================================================\n");

        Ok(())
    }
}