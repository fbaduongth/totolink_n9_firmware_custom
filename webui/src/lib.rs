use std::cell::RefCell;
use std::collections::BTreeMap;

const DEFAULT_OTA_REPO: &str = "https://github.com/fbaduongth/totolink_n9_firmware_custom";
const CURRENT_VERSION_LABEL: &str = "Concard N9 v2.1";
const DEFAULT_BUILD_CODE: &str = "CSB334A-2700-2100";
const DEFAULT_BUILD_DATE: &str = "2026-11-19";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Dashboard,
    Wireless,
    Network,
    Users,
    System,
}

impl Tab {
    fn from_str(s: &str) -> Self {
        match s {
            "wireless" => Tab::Wireless,
            "network" => Tab::Network,
            "users" => Tab::Users,
            "system" => Tab::System,
            _ => Tab::Dashboard,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Tab::Dashboard => "dashboard",
            Tab::Wireless => "wireless",
            Tab::Network => "network",
            Tab::Users => "users",
            Tab::System => "system",
        }
    }
}

#[derive(Clone, Default)]
struct ClientEntry {
    mac: String,
    ip: String,
    hostname: String,
    os_hint: String,
    vap: String, // "Main" or "Guest"
    phy_mode: String,
    bw: String,
    tx_rate_mbps: String,
    rx_rate_mbps: String,
    rssi_dbm: i32,
    signal_pct: i32,
    connect_time: String,
}

#[derive(Clone, Default)]
struct SwitchPortEntry {
    up: bool,
    speed: u32,
    duplex: String,
}

#[derive(Clone, Default)]
struct RrmNeighborEntry {
    bssid: String,
    channel: u32,
    rssi_dbm: i32,
    tag: String,
}

#[derive(Clone, Default)]
struct UserAccount {
    username: String,
    role: String,
    is_primary: bool,
}

#[derive(Clone)]
struct WirelessForm {
    radio_enable: bool,
    ssid: String,
    hidden_ssid: bool,
    auth_mode: String, // "open" or "psk"
    wpa_psk: String,
    show_psk: bool,
    channel: String,       // "0".."13"
    ht_bw: String,         // "0" (20MHz) or "1" (20/40MHz)
    tx_power: String,      // "0".."100"
    pure_ofdm: bool,       // rt_gmode == "5" (Pure OFDM g/n) vs "2" (b/g/n)
    cell_breathing: bool,  // cc_cell_breathing == "1"
    qload_enable: bool,    // cc_qload_enable == "1" (IEEE 802.11e QBSS Load)
    probe_rssi: String,    // cc_probe_rssi (default "-82" dBm, 0 = Off)
    rrm_80211k: bool,
    ft_80211r: bool,
    ft_mdid: String,
    kick_rssi: String,
    assoc_rssi: String,
    dtim: String,
    ap_isolate: bool,
    guest_enable: bool,
    guest_ssid: String,
    guest_auth_mode: String,
    guest_wpa_psk: String,
    guest_show_psk: bool,
    guest_lan_isolate: bool,
    guest_ap_isolate: bool,
    guest_vlan_id: String,
}

impl Default for WirelessForm {
    fn default() -> Self {
        Self {
            radio_enable: true,
            ssid: String::from("Concard-N9"),
            hidden_ssid: false,
            auth_mode: String::from("open"),
            wpa_psk: String::new(),
            show_psk: false,
            channel: String::from("0"),
            ht_bw: String::from("0"),
            tx_power: String::from("100"),
            pure_ofdm: true,
            cell_breathing: true,
            qload_enable: true,
            probe_rssi: String::from("-82"),
            rrm_80211k: true,
            ft_80211r: true,
            ft_mdid: String::from("CC"),
            kick_rssi: String::from("0"),
            assoc_rssi: String::from("-84"),
            dtim: String::from("2"),
            ap_isolate: false,
            guest_enable: false,
            guest_ssid: String::from("Concard-Guest"),
            guest_auth_mode: String::from("open"),
            guest_wpa_psk: String::new(),
            guest_show_psk: false,
            guest_lan_isolate: true,
            guest_ap_isolate: true,
            guest_vlan_id: String::from("0"),
        }
    }
}

#[derive(Clone)]
struct NetworkForm {
    hostname: String,
    device_loc: String,
    dhcp_client: bool, // lan_proto_x == "1"
    ipaddr: String,
    netmask: String,
    gateway: String,
    dns1: String,
    dns2: String,
    fair_queue: bool,  // cc_fair_queue == "1"
    l2_shield: bool,   // cc_l2_shield == "1"
    ctrl_url: String,  // cc_ctrl_url
}

impl Default for NetworkForm {
    fn default() -> Self {
        Self {
            hostname: String::from("Concard-N9"),
            device_loc: String::new(),
            dhcp_client: true,
            ipaddr: String::from("192.168.1.165"),
            netmask: String::from("255.255.255.0"),
            gateway: String::from("192.168.1.1"),
            dns1: String::new(),
            dns2: String::new(),
            fair_queue: true,
            l2_shield: true,
            ctrl_url: String::from("http://192.168.1.102:9100"),
        }
    }
}

#[derive(Clone)]
struct SystemForm {
    time_zone: String,
    ntp_server: String,
    led_mode: String,     // "0" = Standard (Green Online / Red Offline), "1" = Bedroom Night Mode
    ota_repo_url: String, // Default: https://github.com/fbaduongth/totolink_n9_firmware_custom
}

impl Default for SystemForm {
    fn default() -> Self {
        Self {
            time_zone: String::from("UCT-7"),
            ntp_server: String::from("time.google.com"),
            led_mode: String::from("0"),
            ota_repo_url: String::from(DEFAULT_OTA_REPO),
        }
    }
}

#[derive(Clone, Default)]
struct OtaStatus {
    checking: bool,
    checked: bool,
    ok: bool,
    has_update: bool,
    no_release_yet: bool,
    via_browser_relay: bool,
    latest_version: String,
    latest_build: String,
    file_name: String,
    download_url: String,
    sha256: String,
    size_bytes: u64,
    notes: String,
    error: String,
}

#[derive(Clone)]
struct UserManageForm {
    edit_mode: bool,
    edit_is_primary: bool,
    username: String,
    role: String,
    old_password: String,
    password: String,
    confirm_password: String,
    show_password: bool,
}

impl Default for UserManageForm {
    fn default() -> Self {
        Self {
            edit_mode: false,
            edit_is_primary: false,
            username: String::new(),
            role: String::from("viewer"),
            old_password: String::new(),
            password: String::new(),
            confirm_password: String::new(),
            show_password: false,
        }
    }
}

struct LiveMetrics {
    firmver: String,
    build_code: String,
    build_date: String,
    hostname: String,
    device_loc: String,
    ap_mac: String,
    guest_mac: String,
    live_ip: String,
    live_mask: String,
    live_gw: String,
    live_channel: u32,
    uptime_days: u64,
    uptime_hours: u64,
    uptime_mins: u64,
    load_avg: String,
    ram_total_kb: u64,
    ram_used_kb: u64,
    ram_free_kb: u64,
    wlan_rx_bytes: u64,
    wlan_tx_bytes: u64,
    lan_rx_bytes: u64,
    lan_tx_bytes: u64,
    channel_busy_pct: u32,
    false_cca_1s: u64,
    temp_c: i32,
    locate_remaining_sec: u32,
    breathing_active: bool,
    link_up: bool,
    max_speed_mbps: u32,
    degraded_10m: bool,
    gateway_reachable: bool,
    fallback_alias_active: bool,
    ports: Vec<SwitchPortEntry>,
    clients: Vec<ClientEntry>,
    rrm_neighbors: Vec<RrmNeighborEntry>,
    ip_map: BTreeMap<String, (String, String)>,
}

impl Default for LiveMetrics {
    fn default() -> Self {
        Self {
            firmver: String::new(),
            build_code: String::from(DEFAULT_BUILD_CODE),
            build_date: String::from(DEFAULT_BUILD_DATE),
            hostname: String::new(),
            device_loc: String::new(),
            ap_mac: String::new(),
            guest_mac: String::new(),
            live_ip: String::new(),
            live_mask: String::new(),
            live_gw: String::new(),
            live_channel: 0,
            uptime_days: 0,
            uptime_hours: 0,
            uptime_mins: 0,
            load_avg: String::new(),
            ram_total_kb: 0,
            ram_used_kb: 0,
            ram_free_kb: 0,
            wlan_rx_bytes: 0,
            wlan_tx_bytes: 0,
            lan_rx_bytes: 0,
            lan_tx_bytes: 0,
            channel_busy_pct: 0,
            false_cca_1s: 0,
            temp_c: 0,
            locate_remaining_sec: 0,
            breathing_active: false,
            link_up: true,
            max_speed_mbps: 100,
            degraded_10m: false,
            gateway_reachable: true,
            fallback_alias_active: false,
            ports: Vec::new(),
            clients: Vec::new(),
            rrm_neighbors: Vec::new(),
            ip_map: BTreeMap::new(),
        }
    }
}

struct AppState {
    tab: Tab,
    initialized: bool,
    auth_checked: bool,
    authenticated: bool,
    auth_user: String,
    auth_role: String,
    primary_user: String,
    login_user: String,
    login_pass: String,
    login_show_pass: bool,
    login_busy: bool,
    users: Vec<UserAccount>,
    user_form: UserManageForm,
    live: LiveMetrics,
    wireless: WirelessForm,
    network: NetworkForm,
    system: SystemForm,
    ota: OtaStatus,
    notice_kind: String, // "", "ok", "err", "info"
    notice_msg: String,
    out_buf: Vec<u8>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            tab: Tab::Dashboard,
            initialized: false,
            auth_checked: false,
            authenticated: false,
            auth_user: String::new(),
            auth_role: String::from("admin"),
            primary_user: String::from("admin"),
            login_user: String::from("admin"),
            login_pass: String::new(),
            login_show_pass: false,
            login_busy: false,
            users: Vec::new(),
            user_form: UserManageForm::default(),
            live: LiveMetrics::default(),
            wireless: WirelessForm::default(),
            network: NetworkForm::default(),
            system: SystemForm::default(),
            ota: OtaStatus::default(),
            notice_kind: String::new(),
            notice_msg: String::new(),
            out_buf: Vec::with_capacity(32768),
        }
    }
}

thread_local! {
    static APP: RefCell<AppState> = RefCell::new(AppState::default());
}

fn html_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 8);
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                out.push((h << 4) | l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn url_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 2);
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => {
                out.push('%');
                out.push(hex_char(b >> 4));
                out.push(hex_char(b & 0x0f));
            }
        }
    }
    out
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn hex_char(n: u8) -> char {
    match n {
        0..=9 => (b'0' + n) as char,
        _ => (b'A' + (n - 10)) as char,
    }
}

fn is_valid_ipv4(ip: &str) -> bool {
    let parts: Vec<&str> = ip.trim().split('.').collect();
    if parts.len() != 4 {
        return false;
    }
    for p in parts {
        if p.is_empty() || (p.len() > 1 && p.starts_with('0')) {
            return false;
        }
        if p.parse::<u8>().is_err() {
            return false;
        }
    }
    true
}

fn is_valid_username(u: &str) -> bool {
    let len = u.len();
    if !(1..=32).contains(&len) {
        return false;
    }
    u.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
}

fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let b = bytes as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}

fn extract_num_after(haystack: &str, key: &str) -> Option<u64> {
    let idx = haystack.find(key)?;
    let rest = &haystack[idx + key.len()..];
    let trimmed = rest.trim_start_matches(|c: char| c.is_whitespace() || c == ':');
    if let Some(hex_str) = trimmed.strip_prefix("0x") {
        let end = hex_str
            .find(|c: char| !c.is_ascii_hexdigit())
            .unwrap_or(hex_str.len());
        u64::from_str_radix(&hex_str[..end], 16).ok()
    } else {
        let end = trimmed
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(trimmed.len());
        trimmed[..end].parse::<u64>().ok()
    }
}

fn json_get_str(json: &str, key: &str) -> Option<String> {
    let pat = format!("\"{}\"", key);
    let mut search = json;
    while let Some(pos) = search.find(&pat) {
        let after_key = &search[pos + pat.len()..];
        let trimmed = after_key.trim_start();
        if let Some(after_colon) = trimmed.strip_prefix(':') {
            let val_part = after_colon.trim_start();
            if let Some(rest) = val_part.strip_prefix('"') {
                let mut out = String::new();
                let mut chars = rest.chars();
                while let Some(ch) = chars.next() {
                    if ch == '\\' {
                        if let Some(esc) = chars.next() {
                            match esc {
                                'n' => out.push('\n'),
                                'r' => out.push('\r'),
                                't' => out.push('\t'),
                                other => out.push(other),
                            }
                        }
                    } else if ch == '"' {
                        return Some(out);
                    } else {
                        out.push(ch);
                    }
                }
                return Some(out);
            }
        }
        search = after_key;
    }
    None
}

fn json_get_bool(json: &str, key: &str) -> Option<bool> {
    let pat = format!("\"{}\"", key);
    let mut search = json;
    while let Some(pos) = search.find(&pat) {
        let after_key = &search[pos + pat.len()..];
        let trimmed = after_key.trim_start();
        if let Some(after_colon) = trimmed.strip_prefix(':') {
            let val_part = after_colon.trim_start();
            if val_part.starts_with("true") {
                return Some(true);
            } else if val_part.starts_with("false") {
                return Some(false);
            }
        }
        search = after_key;
    }
    None
}

fn json_get_i64(json: &str, key: &str) -> Option<i64> {
    let pat = format!("\"{}\"", key);
    let mut search = json;
    while let Some(pos) = search.find(&pat) {
        let after_key = &search[pos + pat.len()..];
        let trimmed = after_key.trim_start();
        if let Some(after_colon) = trimmed.strip_prefix(':') {
            let val_part = after_colon.trim_start().trim_start_matches('"');
            let mut end = 0;
            for (idx, b) in val_part.bytes().enumerate() {
                if (idx == 0 && b == b'-') || b.is_ascii_digit() {
                    end = idx + 1;
                } else {
                    break;
                }
            }
            if end > 0 && &val_part[..end] != "-" {
                if let Ok(v) = val_part[..end].parse::<i64>() {
                    return Some(v);
                }
            }
        }
        search = after_key;
    }
    None
}

fn json_get_u64(json: &str, key: &str) -> Option<u64> {
    json_get_i64(json, key).and_then(|v| if v >= 0 { Some(v as u64) } else { None })
}

fn extract_json_array_slice<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let pat = format!("\"{}\"", key);
    let pos = json.find(&pat)?;
    let after = &json[pos + pat.len()..];
    let open = after.find('[')?;
    let bytes = after.as_bytes();
    let mut depth = 0usize;
    let mut in_str = false;
    let mut esc = false;
    for i in open..bytes.len() {
        let b = bytes[i];
        if in_str {
            if esc {
                esc = false;
            } else if b == b'\\' {
                esc = true;
            } else if b == b'"' {
                in_str = false;
            }
            continue;
        }
        if b == b'"' {
            in_str = true;
        } else if b == b'[' {
            depth += 1;
        } else if b == b']' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(&after[open + 1..i]);
            }
        }
    }
    None
}

fn format_conn_sec(sec: u64) -> String {
    let h = sec / 3600;
    let m = (sec % 3600) / 60;
    let s = sec % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}

fn parse_ports_array(json: &str) -> Vec<SwitchPortEntry> {
    let mut list = Vec::new();
    let Some(arr_body) = extract_json_array_slice(json, "ports") else {
        return list;
    };
    let mut rest = arr_body;
    while let Some(obj_open) = rest.find('{') {
        let after = &rest[obj_open..];
        let Some(obj_close) = after.find('}') else { break };
        let obj_s = &after[..=obj_close];
        let up = json_get_bool(obj_s, "up").unwrap_or(false);
        let speed = json_get_u64(obj_s, "speed").unwrap_or(0) as u32;
        let duplex = json_get_str(obj_s, "duplex").unwrap_or_else(|| String::from("NONE"));
        list.push(SwitchPortEntry {
            up,
            speed,
            duplex,
        });
        rest = &after[obj_close + 1..];
    }
    list
}

fn parse_stations_array(
    json: &str,
    ip_map: &BTreeMap<String, (String, String)>,
) -> Vec<ClientEntry> {
    let mut list = Vec::new();
    let Some(arr_body) = extract_json_array_slice(json, "stations") else {
        return list;
    };
    let mut rest = arr_body;
    while let Some(obj_open) = rest.find('{') {
        let after = &rest[obj_open..];
        let Some(obj_close) = after.find('}') else { break };
        let obj_s = &after[..=obj_close];
        if let Some(mac_raw) = json_get_str(obj_s, "mac") {
            let mac = mac_raw.to_ascii_uppercase();
            let ap_idx = json_get_u64(obj_s, "apIdx").unwrap_or(0);
            let vap = if ap_idx == 1 { "Guest" } else { "Main" };
            let mut ip = json_get_str(obj_s, "ip").unwrap_or_default();
            let mut hostname = json_get_str(obj_s, "hostname").unwrap_or_default();
            let os_hint = json_get_str(obj_s, "osHint").unwrap_or_default();
            if ip.is_empty() {
                if let Some((m_ip, m_host)) = ip_map.get(&mac) {
                    ip = m_ip.clone();
                    if hostname.is_empty() {
                        hostname = m_host.clone();
                    }
                } else {
                    ip = String::from("—");
                }
            }
            let rssi_dbm = json_get_i64(obj_s, "rssi").unwrap_or(-70) as i32;
            let pct = (((rssi_dbm + 92) as f32 / 0.52).round() as i32).clamp(1, 100);
            let tx_mbps = json_get_u64(obj_s, "txMbps").unwrap_or(65);
            let rx_mbps = json_get_u64(obj_s, "rxMbps").unwrap_or(tx_mbps);
            let bw_num = json_get_u64(obj_s, "bw").unwrap_or(20);
            let mode = json_get_str(obj_s, "mode").unwrap_or_else(|| String::from("802.11n"));
            let conn_sec = json_get_u64(obj_s, "connSec").unwrap_or(0);
            list.push(ClientEntry {
                mac,
                ip,
                hostname,
                os_hint,
                vap: String::from(vap),
                phy_mode: mode,
                bw: format!("{} MHz", bw_num),
                tx_rate_mbps: tx_mbps.to_string(),
                rx_rate_mbps: rx_mbps.to_string(),
                rssi_dbm,
                signal_pct: pct,
                connect_time: format_conn_sec(conn_sec),
            });
        }
        rest = &after[obj_close + 1..];
    }
    list.sort_by(|a, b| b.rssi_dbm.cmp(&a.rssi_dbm));
    list
}

fn parse_users_array(json: &str) -> Vec<UserAccount> {
    let mut list = Vec::new();
    let Some(u_pos) = json.find("\"users\"") else {
        return list;
    };
    let sub = &json[u_pos..];
    let Some(arr_open) = sub.find('[') else {
        return list;
    };
    let Some(arr_close) = sub.rfind(']') else {
        return list;
    };
    if arr_close <= arr_open {
        return list;
    }
    let arr_body = &sub[arr_open + 1..arr_close];
    let mut rest = arr_body;
    while let Some(obj_open) = rest.find('{') {
        let after = &rest[obj_open..];
        let Some(obj_close) = after.find('}') else {
            break;
        };
        let obj_s = &after[..=obj_close];
        if let Some(username) = json_get_str(obj_s, "username") {
            let role = json_get_str(obj_s, "role").unwrap_or_else(|| String::from("viewer"));
            let is_primary = json_get_bool(obj_s, "is_primary").unwrap_or(false);
            if !username.is_empty() {
                list.push(UserAccount {
                    username,
                    role,
                    is_primary,
                });
            }
        }
        rest = &after[obj_close + 1..];
    }
    list
}

fn parse_sys_status_line(sys_line: &str, live: &mut LiveMetrics) {
    if let Some(q1) = sys_line.find("lavg:") {
        let after = &sys_line[q1 + 5..];
        if let Some(s1) = after.find('"') {
            let rem = &after[s1 + 1..];
            if let Some(s2) = rem.find('"') {
                live.load_avg = rem[..s2].to_string();
            }
        }
    }
    if let Some(up_pos) = sys_line.find("uptime:") {
        let sub = &sys_line[up_pos..];
        if let Some(d) = extract_num_after(sub, "days") {
            live.uptime_days = d;
        }
        if let Some(h) = extract_num_after(sub, "hours") {
            live.uptime_hours = h;
        }
        if let Some(m) = extract_num_after(sub, "minutes") {
            live.uptime_mins = m;
        }
    }
    if let Some(ram_pos) = sys_line.find("ram:") {
        let sub = &sys_line[ram_pos..];
        if let Some(t) = extract_num_after(sub, "total") {
            live.ram_total_kb = t;
        }
        if let Some(u) = extract_num_after(sub, "used") {
            live.ram_used_kb = u;
        }
        if let Some(f) = extract_num_after(sub, "free") {
            live.ram_free_kb = f;
        }
    }
}

impl AppState {
    fn is_admin(&self) -> bool {
        self.auth_role != "viewer"
    }

    fn set_output(&mut self, s: String) -> *const u8 {
        self.out_buf = s.into_bytes();
        self.out_buf.as_ptr()
    }

    fn build_badge_str(&self) -> String {
        let code = if self.live.build_code.is_empty() {
            DEFAULT_BUILD_CODE
        } else {
            &self.live.build_code
        };
        if self.live.build_date.is_empty() {
            format!("Build {}", html_escape(code))
        } else {
            format!(
                "Build {} ({})",
                html_escape(code),
                html_escape(&self.live.build_date)
            )
        }
    }

    fn ingest_auth_json(&mut self, raw: &str) {
        self.auth_checked = true;
        self.login_busy = false;
        if raw.trim().is_empty() {
            return;
        }
        if let Some(h) = json_get_str(raw, "hostname") {
            if !h.is_empty() {
                if self.live.hostname.is_empty() {
                    self.live.hostname = h.clone();
                }
                if !self.initialized {
                    self.network.hostname = h;
                }
            }
        }
        if let Some(ip) = json_get_str(raw, "ip") {
            if !ip.is_empty() && self.live.live_ip.is_empty() {
                self.live.live_ip = ip;
            }
        }
        if let Some(fw) = json_get_str(raw, "firmver") {
            if !fw.is_empty() && self.live.firmver.is_empty() {
                self.live.firmver = fw;
            }
        }
        if let Some(auth) = json_get_bool(raw, "authenticated") {
            let was_auth = self.authenticated;
            self.authenticated = auth;
            if auth {
                if let Some(u) = json_get_str(raw, "user") {
                    if !u.is_empty() {
                        self.auth_user = u.clone();
                        self.login_user = u;
                    }
                }
                if let Some(r) = json_get_str(raw, "role") {
                    if !r.is_empty() {
                        self.auth_role = r;
                    }
                }
                self.login_pass.clear();
                self.login_show_pass = false;
                if !was_auth {
                    self.notice_kind.clear();
                    self.notice_msg.clear();
                }
            } else {
                self.auth_user.clear();
                self.initialized = false;
            }
        }
    }

    fn ingest_users_json(&mut self, raw: &str) {
        if raw.trim().is_empty() {
            return;
        }
        if let Some(cu) = json_get_str(raw, "current_user") {
            if !cu.is_empty() {
                self.auth_user = cu;
            }
        }
        if let Some(cr) = json_get_str(raw, "current_role") {
            if !cr.is_empty() {
                self.auth_role = cr;
            }
        }
        if let Some(pu) = json_get_str(raw, "primary_user") {
            if !pu.is_empty() {
                self.primary_user = pu;
            }
        }
        self.users = parse_users_array(raw);
    }

    fn ingest_ota_json(&mut self, raw: &str) {
        self.ota.checking = false;
        self.ota.checked = true;
        if raw.trim().is_empty() {
            self.ota.ok = false;
            self.ota.error = String::from("Empty response while checking GitHub releases.");
            return;
        }
        self.ota.ok = json_get_bool(raw, "ok").unwrap_or(false);
        self.ota.has_update = json_get_bool(raw, "hasUpdate").unwrap_or(false);
        self.ota.no_release_yet = json_get_bool(raw, "noReleaseYet").unwrap_or(false);
        self.ota.via_browser_relay = json_get_bool(raw, "viaBrowserRelay").unwrap_or(false);
        self.ota.latest_version = json_get_str(raw, "latestVersion").unwrap_or_default();
        self.ota.latest_build = json_get_str(raw, "latestBuild").unwrap_or_default();
        self.ota.file_name = json_get_str(raw, "fileName").unwrap_or_default();
        self.ota.download_url = json_get_str(raw, "downloadUrl").unwrap_or_default();
        self.ota.sha256 = json_get_str(raw, "sha256").unwrap_or_default();
        self.ota.size_bytes = json_get_u64(raw, "sizeBytes").unwrap_or(0);
        self.ota.notes = json_get_str(raw, "notes").unwrap_or_default();
        self.ota.error = json_get_str(raw, "error").unwrap_or_default();
        if let Some(repo) = json_get_str(raw, "repoUrl") {
            if !repo.is_empty() {
                self.system.ota_repo_url = repo;
            }
        }
    }

    fn select_edit_user(&mut self, username: &str) {
        if let Some(u) = self.users.iter().find(|x| x.username == username).cloned() {
            self.user_form.edit_mode = true;
            self.user_form.edit_is_primary = u.is_primary;
            self.user_form.username = u.username;
            self.user_form.role = u.role;
            self.user_form.password.clear();
            self.user_form.confirm_password.clear();
            self.user_form.old_password.clear();
        }
    }

    fn reset_user_form(&mut self) {
        self.user_form = UserManageForm::default();
    }

    fn ingest_state_kv(&mut self, raw: &str, sync_forms: bool) {
        let should_sync = sync_forms || !self.initialized;
        let mut lan_ip_cfg = String::new();
        let mut lan_mask_cfg = String::new();
        let mut lan_gw_cfg = String::new();
        let mut lan_ip_rt = String::new();
        let mut lan_mask_rt = String::new();
        let mut lan_gw_rt = String::new();
        let mut in_rrm_block = false;
        let mut rrm_list = Vec::new();
        let mut saw_rrm_marker = false;

        for line in raw.lines() {
            let ln = line.trim();
            if ln == "__RRM_BEGIN__" {
                in_rrm_block = true;
                saw_rrm_marker = true;
                continue;
            }
            if ln == "__RRM_END__" {
                in_rrm_block = false;
                continue;
            }
            if in_rrm_block {
                if ln.is_empty() {
                    continue;
                }
                let cols: Vec<&str> = ln.split_whitespace().collect();
                if cols.len() >= 2 && cols[0].len() == 17 && cols[0].contains(':') {
                    let bssid = cols[0].to_ascii_uppercase();
                    let channel = cols[1].parse::<u32>().unwrap_or(0);
                    let rssi_dbm = if cols.len() >= 3 {
                        cols[2].parse::<i32>().unwrap_or(-68)
                    } else {
                        -68
                    };
                    let tag = if cols.len() >= 4 && cols[3] != "-" {
                        cols[3..].join(" ")
                    } else {
                        String::new()
                    };
                    if (1..=13).contains(&channel) {
                        rrm_list.push(RrmNeighborEntry {
                            bssid,
                            channel,
                            rssi_dbm,
                            tag,
                        });
                    }
                }
                continue;
            }
            if ln.is_empty() {
                continue;
            }
            let Some(eq) = ln.find('=') else { continue };
            let k = ln[..eq].trim();
            let v = ln[eq + 1..].trim();

            match k {
                "firmver" => {
                    if !v.is_empty() {
                        self.live.firmver = v.to_string();
                    }
                }
                "sys_status" => parse_sys_status_line(v, &mut self.live),
                "lan_ipaddr_t" => lan_ip_rt = v.to_string(),
                "lan_netmask_t" => lan_mask_rt = v.to_string(),
                "lan_gateway_t" => lan_gw_rt = v.to_string(),
                "lan_ipaddr" => lan_ip_cfg = v.to_string(),
                "lan_netmask" => lan_mask_cfg = v.to_string(),
                "lan_gateway" => lan_gw_cfg = v.to_string(),
                "rt_radio_x" if should_sync => self.wireless.radio_enable = v != "0",
                "rt_ssid2" if should_sync => {
                    let dec = percent_decode(v);
                    if !dec.is_empty() {
                        self.wireless.ssid = dec;
                    }
                }
                "rt_closed" if should_sync => self.wireless.hidden_ssid = v == "1",
                "rt_auth_mode" if should_sync => {
                    self.wireless.auth_mode = if v == "psk" {
                        String::from("psk")
                    } else {
                        String::from("open")
                    };
                }
                "rt_wpa_psk_org" if should_sync => {
                    self.wireless.wpa_psk = percent_decode(v);
                }
                "rt_channel" if should_sync => {
                    self.wireless.channel = if v.is_empty() {
                        String::from("0")
                    } else {
                        v.to_string()
                    };
                }
                "rt_HT_BW" if should_sync => {
                    self.wireless.ht_bw = if v == "1" {
                        String::from("1")
                    } else {
                        String::from("0")
                    };
                }
                "rt_TxPower" if should_sync => {
                    if !v.is_empty() {
                        self.wireless.tx_power = v.to_string();
                    }
                }
                "rt_gmode" if should_sync => {
                    if !v.is_empty() {
                        self.wireless.pure_ofdm = v != "2" && v != "0" && v != "1";
                    }
                }
                "cc_cell_breathing" if should_sync => {
                    if !v.is_empty() {
                        self.wireless.cell_breathing = v != "0";
                    }
                }
                "cc_qload_enable" if should_sync => {
                    if !v.is_empty() {
                        self.wireless.qload_enable = v != "0";
                    }
                }
                "cc_probe_rssi" if should_sync => {
                    if !v.is_empty() {
                        self.wireless.probe_rssi = v.to_string();
                    }
                }
                "rt_rrm_enable" if should_sync => self.wireless.rrm_80211k = v != "0",
                "rt_ft_enable" if should_sync => self.wireless.ft_80211r = v != "0",
                "rt_ft_mdid" if should_sync => {
                    if !v.is_empty() {
                        self.wireless.ft_mdid = v.to_string();
                    }
                }
                "rt_KickStaRssiLow" if should_sync => {
                    if !v.is_empty() {
                        self.wireless.kick_rssi = v.to_string();
                    }
                }
                "rt_AssocReqRssiThres" if should_sync => {
                    if !v.is_empty() {
                        self.wireless.assoc_rssi = v.to_string();
                    }
                }
                "rt_dtim" if should_sync => {
                    if !v.is_empty() {
                        self.wireless.dtim = v.to_string();
                    }
                }
                "rt_ap_isolate" if should_sync => self.wireless.ap_isolate = v == "1",
                "rt_guest_enable" if should_sync => self.wireless.guest_enable = v == "1",
                "rt_guest_ssid2" if should_sync => {
                    let dec = percent_decode(v);
                    if !dec.is_empty() {
                        self.wireless.guest_ssid = dec;
                    }
                }
                "rt_guest_auth_mode" if should_sync => {
                    self.wireless.guest_auth_mode = if v == "psk" {
                        String::from("psk")
                    } else {
                        String::from("open")
                    };
                }
                "rt_guest_wpa_psk_org" if should_sync => {
                    self.wireless.guest_wpa_psk = percent_decode(v);
                }
                "rt_guest_lan_isolate" if should_sync => self.wireless.guest_lan_isolate = v != "0",
                "rt_guest_ap_isolate" if should_sync => self.wireless.guest_ap_isolate = v != "0",
                "cc_guest_vlan" if should_sync => {
                    self.wireless.guest_vlan_id = if v.is_empty() {
                        String::from("0")
                    } else {
                        v.to_string()
                    };
                }
                "computer_name" => {
                    let dec = percent_decode(v);
                    if !dec.is_empty() {
                        self.live.hostname = dec.clone();
                        if should_sync {
                            self.network.hostname = dec;
                        }
                    }
                }
                "cc_device_loc" => {
                    let dec = percent_decode(v);
                    self.live.device_loc = dec.clone();
                    if should_sync {
                        self.network.device_loc = dec;
                    }
                }
                "lan_proto_x" if should_sync => self.network.dhcp_client = v != "0",
                "lan_dns1" if should_sync => self.network.dns1 = v.to_string(),
                "lan_dns2" if should_sync => self.network.dns2 = v.to_string(),
                "cc_fair_queue" if should_sync => {
                    if !v.is_empty() {
                        self.network.fair_queue = v != "0";
                    }
                }
                "cc_l2_shield" if should_sync => {
                    if !v.is_empty() {
                        self.network.l2_shield = v != "0";
                    }
                }
                "cc_ctrl_url" if should_sync => {
                    let dec = percent_decode(v);
                    if !dec.is_empty() {
                        self.network.ctrl_url = dec;
                    }
                }
                "http_username" => {
                    if !v.is_empty() {
                        self.primary_user = v.to_string();
                    }
                }
                "time_zone" if should_sync => {
                    if !v.is_empty() {
                        self.system.time_zone = v.to_string();
                    }
                }
                "ntp_server0" if should_sync => {
                    if !v.is_empty() {
                        self.system.ntp_server = v.to_string();
                    }
                }
                "cc_led_mode" if should_sync => {
                    if !v.is_empty() {
                        self.system.led_mode = if v == "1" {
                            String::from("1")
                        } else {
                            String::from("0")
                        };
                    }
                }
                "cc_ota_repo_url" if should_sync => {
                    let dec = percent_decode(v);
                    if !dec.is_empty() {
                        self.system.ota_repo_url = dec;
                    }
                }
                _ => {}
            }
        }

        self.live.live_ip = if !lan_ip_rt.is_empty() && lan_ip_rt != "0.0.0.0" {
            lan_ip_rt
        } else {
            lan_ip_cfg.clone()
        };
        self.live.live_mask = if !lan_mask_rt.is_empty() && lan_mask_rt != "0.0.0.0" {
            lan_mask_rt
        } else {
            lan_mask_cfg.clone()
        };
        self.live.live_gw = if !lan_gw_rt.is_empty() && lan_gw_rt != "0.0.0.0" {
            lan_gw_rt
        } else {
            lan_gw_cfg.clone()
        };

        if saw_rrm_marker {
            self.live.rrm_neighbors = rrm_list;
        }

        if should_sync {
            if !lan_ip_cfg.is_empty() {
                self.network.ipaddr = lan_ip_cfg;
            } else if !self.live.live_ip.is_empty() {
                self.network.ipaddr = self.live.live_ip.clone();
            }
            if !lan_mask_cfg.is_empty() {
                self.network.netmask = lan_mask_cfg;
            }
            if !lan_gw_cfg.is_empty() {
                self.network.gateway = lan_gw_cfg;
            }
            self.initialized = true;
        }
    }

    fn ingest_ipmonitor(&mut self, raw: &str) {
        let Some(start) = raw.find("ipmonitor") else { return };
        let sub = &raw[start..];
        let Some(b_open) = sub.find('[') else { return };
        let Some(b_close) = sub.rfind(']') else { return };
        if b_close <= b_open {
            return;
        }
        let body = &sub[b_open + 1..b_close];
        let mut rest = body;
        while let Some(r_open) = rest.find('[') {
            let after_open = &rest[r_open + 1..];
            let Some(r_close) = after_open.find(']') else { break };
            let row = &after_open[..r_close];
            let cols: Vec<&str> = row.split(',').map(|s| s.trim().trim_matches('"')).collect();
            if cols.len() >= 3 {
                let ip = cols[0].trim();
                let mac = cols[1].trim().to_ascii_uppercase();
                let host = if cols[2] == "null" {
                    String::new()
                } else {
                    percent_decode(cols[2])
                };
                if !ip.is_empty() && mac.len() == 17 {
                    self.live.ip_map.insert(mac, (ip.to_string(), host));
                }
            }
            rest = &after_open[r_close + 1..];
        }

        for cl in &mut self.live.clients {
            if let Some((ip, host)) = self.live.ip_map.get(&cl.mac) {
                cl.ip = ip.clone();
                if cl.hostname.is_empty() {
                    cl.hostname = host.clone();
                }
            }
        }
    }

    fn ingest_wstatus(&mut self, raw: &str) {
        let content = if let Some(t_open) = raw.find("<textarea") {
            let after = &raw[t_open..];
            if let Some(gt) = after.find('>') {
                let inner = &after[gt + 1..];
                if let Some(t_close) = inner.find("</textarea>") {
                    &inner[..t_close]
                } else {
                    inner
                }
            } else {
                raw
            }
        } else {
            raw
        };

        let mut clients = Vec::new();
        let mut in_table = false;
        let mut cur_vap = "Main";

        for line in content.lines() {
            let ln = line.trim();
            if ln.is_empty() {
                continue;
            }
            if let Some(idx) = ln.find("MAC (AP Main)") {
                let rem = &ln[idx..];
                if let Some(colon) = rem.find(':') {
                    let mac_cand = rem[colon + 1..].trim();
                    if mac_cand.len() >= 17 {
                        self.live.ap_mac = mac_cand[..17].to_ascii_uppercase();
                    }
                }
                continue;
            }
            if let Some(idx) = ln.find("MAC (AP Guest)") {
                let rem = &ln[idx..];
                if let Some(colon) = rem.find(':') {
                    let mac_cand = rem[colon + 1..].trim();
                    if mac_cand.len() >= 17 {
                        self.live.guest_mac = mac_cand[..17].to_ascii_uppercase();
                    }
                }
                continue;
            }
            if let Some(idx) = ln.find("Channel Main") {
                let rem = &ln[idx..];
                if let Some(colon) = rem.find(':') {
                    if let Ok(ch) = rem[colon + 1..].trim().parse::<u32>() {
                        self.live.live_channel = ch;
                    }
                }
                continue;
            }
            if ln.starts_with("AP Main Stations List") {
                cur_vap = "Main";
                in_table = false;
                continue;
            }
            if ln.starts_with("AP Guest Stations List") {
                cur_vap = "Guest";
                in_table = false;
                continue;
            }
            if ln.starts_with("MAC") && ln.contains("PhyMode") && ln.contains("BW") {
                in_table = true;
                continue;
            }
            if !in_table || ln.starts_with("---") || ln.starts_with("AP ") {
                continue;
            }

            let cols: Vec<&str> = ln.split_whitespace().collect();
            if cols.len() < 9 {
                continue;
            }
            let mac_raw = cols[0];
            if mac_raw.len() != 17 || mac_raw.chars().filter(|&c| c == ':').count() != 5 {
                continue;
            }
            let mac = mac_raw.to_ascii_uppercase();
            let raw_phy = cols[1].to_ascii_uppercase();
            let phy_mode = if raw_phy.contains("CCK") {
                "802.11b"
            } else if raw_phy.contains("OFDM") {
                "802.11g"
            } else {
                "802.11n"
            };
            let bw = cols[2].replace('M', " MHz");
            let tx_rate = cols[7].trim_end_matches('M').to_string();
            let rssi_dbm = cols[8].parse::<i32>().unwrap_or(-70);
            let pct = (((rssi_dbm + 92) as f32 / 0.52).round() as i32).clamp(1, 100);
            let conn_time = cols.last().copied().unwrap_or("00:00:00").to_string();

            let (ip, hostname) = self
                .live
                .ip_map
                .get(&mac)
                .cloned()
                .unwrap_or_else(|| (String::from("—"), String::new()));

            clients.push(ClientEntry {
                mac,
                ip,
                hostname,
                os_hint: String::new(),
                vap: cur_vap.to_string(),
                phy_mode: phy_mode.to_string(),
                bw,
                tx_rate_mbps: tx_rate.clone(),
                rx_rate_mbps: tx_rate,
                rssi_dbm,
                signal_pct: pct,
                connect_time: conn_time,
            });
        }

        clients.sort_by(|a, b| b.rssi_dbm.cmp(&a.rssi_dbm));
        self.live.clients = clients;
    }

    fn ingest_netdev(&mut self, raw: &str) {
        if let Some(pos) = raw.find("'WLAN2_AP0'") {
            let sub = &raw[pos..];
            if let Some(rx) = extract_num_after(sub, "rx") {
                self.live.wlan_rx_bytes = rx;
            }
            if let Some(tx) = extract_num_after(sub, "tx") {
                self.live.wlan_tx_bytes = tx;
            }
        }
        if let Some(pos) = raw.find("'LAN'") {
            let sub = &raw[pos..];
            if let Some(rx) = extract_num_after(sub, "rx") {
                self.live.lan_rx_bytes = rx;
            }
            if let Some(tx) = extract_num_after(sub, "tx") {
                self.live.lan_tx_bytes = tx;
            }
        }
    }

    fn ingest_locate_json(&mut self, raw: &str) {
        if raw.trim().is_empty() {
            return;
        }
        if let Some(rem) = json_get_u64(raw, "remainingSec") {
            self.live.locate_remaining_sec = rem.min(300) as u32;
        } else if json_get_bool(raw, "active") == Some(false) {
            self.live.locate_remaining_sec = 0;
        }
    }

    fn ingest_v2_telemetry(&mut self, raw: &str, sync_forms: bool) {
        if raw.trim().is_empty() {
            return;
        }
        let should_sync = sync_forms || !self.initialized;

        if let Some(fw) = json_get_str(raw, "firmver_sub") {
            if !fw.is_empty() {
                self.live.firmver = fw;
            }
        }
        if let Some(bc) = json_get_str(raw, "buildCode") {
            if !bc.is_empty() {
                self.live.build_code = bc;
            }
        }
        if let Some(bd) = json_get_str(raw, "buildDate") {
            if !bd.is_empty() {
                self.live.build_date = bd;
            }
        }
        if let Some(hn) = json_get_str(raw, "computer_name") {
            if !hn.is_empty() {
                self.live.hostname = hn.clone();
                if should_sync {
                    self.network.hostname = hn;
                }
            }
        }
        if let Some(loc) = json_get_str(raw, "cc_device_loc") {
            self.live.device_loc = loc.clone();
            if should_sync {
                self.network.device_loc = loc;
            }
        }
        if let Some(mac) = json_get_str(raw, "apMac") {
            if !mac.is_empty() {
                self.live.ap_mac = mac;
            }
        }
        if let Some(gmac) = json_get_str(raw, "guestMac") {
            self.live.guest_mac = gmac;
        }
        if let Some(ip) = json_get_str(raw, "lan_ipaddr_t") {
            if !ip.is_empty() && ip != "0.0.0.0" {
                self.live.live_ip = ip;
            }
        }
        if let Some(mask) = json_get_str(raw, "lan_netmask_t") {
            if !mask.is_empty() && mask != "0.0.0.0" {
                self.live.live_mask = mask;
            }
        }
        if let Some(gw) = json_get_str(raw, "lan_gateway_t") {
            if !gw.is_empty() && gw != "0.0.0.0" {
                self.live.live_gw = gw;
            }
        }
        if let Some(gok) = json_get_bool(raw, "gatewayReachable") {
            self.live.gateway_reachable = gok;
        }
        if let Some(fba) = json_get_bool(raw, "fallbackAliasActive") {
            self.live.fallback_alias_active = fba;
        }
        if let Some(up_sec) = json_get_u64(raw, "uptimeSec") {
            self.live.uptime_days = up_sec / 86400;
            self.live.uptime_hours = (up_sec % 86400) / 3600;
            self.live.uptime_mins = (up_sec % 3600) / 60;
        }
        if let Some(lavg) = json_get_str(raw, "loadAvg") {
            if !lavg.is_empty() {
                self.live.load_avg = lavg;
            }
        }
        if let Some(mt) = json_get_u64(raw, "memTotalKb") {
            self.live.ram_total_kb = mt;
        }
        if let Some(mu) = json_get_u64(raw, "memUsedKb") {
            self.live.ram_used_kb = mu;
        }
        if let Some(mf) = json_get_u64(raw, "memFreeKb") {
            self.live.ram_free_kb = mf;
        }
        if let Some(ch) = json_get_u64(raw, "liveChannel") {
            if ch > 0 {
                self.live.live_channel = ch as u32;
            }
        }
        if should_sync {
            if let Some(gm) = json_get_u64(raw, "rt_gmode") {
                self.wireless.pure_ofdm = gm != 2 && gm != 0 && gm != 1;
            }
            if let Some(cb) = json_get_u64(raw, "cc_cell_breathing") {
                self.wireless.cell_breathing = cb != 0;
            }
            if let Some(ql) = json_get_u64(raw, "cc_qload_enable") {
                self.wireless.qload_enable = ql != 0;
            }
            if let Some(pr) = json_get_i64(raw, "cc_probe_rssi") {
                self.wireless.probe_rssi = pr.to_string();
            }
            if let Some(fq) = json_get_u64(raw, "cc_fair_queue") {
                self.network.fair_queue = fq != 0;
            }
            if let Some(l2s) = json_get_u64(raw, "cc_l2_shield") {
                self.network.l2_shield = l2s != 0;
            }
            if let Some(lm) = json_get_u64(raw, "cc_led_mode") {
                self.system.led_mode = if lm == 1 {
                    String::from("1")
                } else {
                    String::from("0")
                };
            }
            if let Some(cu) = json_get_str(raw, "cc_ctrl_url") {
                if !cu.is_empty() {
                    self.network.ctrl_url = cu;
                }
            }
            if let Some(oru) = json_get_str(raw, "cc_ota_repo_url") {
                if !oru.is_empty() {
                    self.system.ota_repo_url = oru;
                }
            }
        }
        if let Some(wrx) = json_get_u64(raw, "wlanRx") {
            self.live.wlan_rx_bytes = wrx;
        }
        if let Some(wtx) = json_get_u64(raw, "wlanTx") {
            self.live.wlan_tx_bytes = wtx;
        }
        if let Some(lrx) = json_get_u64(raw, "lanRx") {
            self.live.lan_rx_bytes = lrx;
        }
        if let Some(ltx) = json_get_u64(raw, "lanTx") {
            self.live.lan_tx_bytes = ltx;
        }
        if let Some(busy) = json_get_u64(raw, "channelBusyPct") {
            self.live.channel_busy_pct = busy.min(100) as u32;
        }
        if let Some(fcca) = json_get_u64(raw, "falseCca1s") {
            self.live.false_cca_1s = fcca;
        }
        if let Some(tc) = json_get_i64(raw, "tempC") {
            self.live.temp_c = tc as i32;
        }
        if let Some(rem) = json_get_u64(raw, "locateRemainingSec") {
            self.live.locate_remaining_sec = rem.min(300) as u32;
        }
        if let Some(ba) = json_get_u64(raw, "breathingActive") {
            self.live.breathing_active = ba != 0;
        }
        if let Some(lu) = json_get_bool(raw, "linkUp") {
            self.live.link_up = lu;
        }
        if let Some(ms) = json_get_u64(raw, "maxSpeedMbps") {
            self.live.max_speed_mbps = ms as u32;
        }
        if let Some(deg) = json_get_bool(raw, "degraded10M") {
            self.live.degraded_10m = deg;
        }
        let ports = parse_ports_array(raw);
        if !ports.is_empty() {
            self.live.ports = ports;
        }
        if raw.contains("\"stations\"") {
            self.live.clients = parse_stations_array(raw, &self.live.ip_map);
        }
    }

    fn update_field(&mut self, key: &str, val: &str) {
        match key {
            "login_user" => self.login_user = val.to_string(),
            "login_pass" => self.login_pass = val.to_string(),
            "login_show_pass" => self.login_show_pass = !self.login_show_pass,

            "um_username" => self.user_form.username = val.to_string(),
            "um_role" => self.user_form.role = val.to_string(),
            "um_old_pass" => self.user_form.old_password = val.to_string(),
            "um_pass" => self.user_form.password = val.to_string(),
            "um_pass2" => self.user_form.confirm_password = val.to_string(),
            "um_show_pass" => self.user_form.show_password = !self.user_form.show_password,

            "w_radio" => self.wireless.radio_enable = val == "1" || val == "true",
            "w_ssid" => self.wireless.ssid = val.to_string(),
            "w_hidden" => self.wireless.hidden_ssid = val == "1" || val == "true",
            "w_auth" => self.wireless.auth_mode = val.to_string(),
            "w_psk" => self.wireless.wpa_psk = val.to_string(),
            "w_show_psk" => self.wireless.show_psk = !self.wireless.show_psk,
            "w_channel" => self.wireless.channel = val.to_string(),
            "w_bw" => self.wireless.ht_bw = val.to_string(),
            "w_txpower" => self.wireless.tx_power = val.to_string(),
            "w_ofdm" => self.wireless.pure_ofdm = val == "1" || val == "true",
            "w_breathing" => self.wireless.cell_breathing = val == "1" || val == "true",
            "w_qload" => self.wireless.qload_enable = val == "1" || val == "true",
            "w_probe_rssi" => self.wireless.probe_rssi = val.to_string(),
            "w_rrm" => self.wireless.rrm_80211k = val == "1" || val == "true",
            "w_ft" => self.wireless.ft_80211r = val == "1" || val == "true",
            "w_mdid" => self.wireless.ft_mdid = val.to_string(),
            "w_kick" => self.wireless.kick_rssi = val.to_string(),
            "w_assoc" => self.wireless.assoc_rssi = val.to_string(),
            "w_dtim" => self.wireless.dtim = val.to_string(),
            "w_isolate" => self.wireless.ap_isolate = val == "1" || val == "true",
            "wg_enable" => self.wireless.guest_enable = val == "1" || val == "true",
            "wg_ssid" => self.wireless.guest_ssid = val.to_string(),
            "wg_auth" => self.wireless.guest_auth_mode = val.to_string(),
            "wg_psk" => self.wireless.guest_wpa_psk = val.to_string(),
            "wg_show_psk" => self.wireless.guest_show_psk = !self.wireless.guest_show_psk,
            "wg_lan_iso" => self.wireless.guest_lan_isolate = val == "1" || val == "true",
            "wg_ap_iso" => self.wireless.guest_ap_isolate = val == "1" || val == "true",
            "wg_vlan" => self.wireless.guest_vlan_id = val.to_string(),

            "n_hostname" => self.network.hostname = val.to_string(),
            "n_loc" => self.network.device_loc = val.to_string(),
            "n_dhcp" => self.network.dhcp_client = val == "1" || val == "true",
            "n_ip" => self.network.ipaddr = val.to_string(),
            "n_mask" => self.network.netmask = val.to_string(),
            "n_gw" => self.network.gateway = val.to_string(),
            "n_dns1" => self.network.dns1 = val.to_string(),
            "n_dns2" => self.network.dns2 = val.to_string(),
            "n_fair_q" => self.network.fair_queue = val == "1" || val == "true",
            "n_l2_shield" => self.network.l2_shield = val == "1" || val == "true",
            "n_ctrl_url" => self.network.ctrl_url = val.to_string(),

            "s_tz" => self.system.time_zone = val.to_string(),
            "s_ntp" => self.system.ntp_server = val.to_string(),
            "s_led_mode" => self.system.led_mode = val.to_string(),
            "s_ota_repo" => self.system.ota_repo_url = val.to_string(),
            _ => {}
        }
    }

    fn build_login_payload(&self) -> Result<String, String> {
        let user = self.login_user.trim();
        if user.is_empty() {
            return Err(String::from("Please enter your username."));
        }
        if self.login_pass.is_empty() {
            return Err(String::from("Please enter your password."));
        }
        let pairs = [
            ("action", String::from("login")),
            ("username", user.to_string()),
            ("password", self.login_pass.clone()),
        ];
        Ok(encode_form(&pairs))
    }

    fn build_user_save_payload(&mut self) -> Result<String, String> {
        if !self.is_admin() {
            if self.user_form.old_password.is_empty() {
                return Err(String::from("Please enter your current password."));
            }
            if self.user_form.password.len() < 4 || self.user_form.password.len() > 64 {
                return Err(String::from("New password must be between 4 and 64 characters."));
            }
            if self.user_form.password != self.user_form.confirm_password {
                return Err(String::from("New password and confirmation do not match."));
            }
            let pairs = [
                ("action", String::from("self_passwd")),
                ("old_password", self.user_form.old_password.clone()),
                ("password", self.user_form.password.clone()),
            ];
            self.user_form.old_password.clear();
            self.user_form.password.clear();
            self.user_form.confirm_password.clear();
            return Ok(encode_form(&pairs));
        }

        let user = self.user_form.username.trim().to_string();
        if !is_valid_username(&user) {
            return Err(String::from(
                "Username must be 1–32 characters (letters, digits, '.', '_', or '-').",
            ));
        }
        let role = if self.user_form.edit_is_primary || self.user_form.role == "admin" {
            String::from("admin")
        } else {
            String::from("viewer")
        };

        if self.user_form.edit_mode {
            if !self.user_form.password.is_empty() || !self.user_form.confirm_password.is_empty() {
                if self.user_form.password.len() < 4 || self.user_form.password.len() > 64 {
                    return Err(String::from("New password must be between 4 and 64 characters."));
                }
                if self.user_form.password != self.user_form.confirm_password {
                    return Err(String::from("Password and Confirm Password do not match."));
                }
            }
            let pairs = [
                ("action", String::from("update")),
                ("username", user),
                ("role", role),
                ("password", self.user_form.password.clone()),
            ];
            self.reset_user_form();
            Ok(encode_form(&pairs))
        } else {
            if self.user_form.password.len() < 4 || self.user_form.password.len() > 64 {
                return Err(String::from("Password must be between 4 and 64 characters."));
            }
            if self.user_form.password != self.user_form.confirm_password {
                return Err(String::from("Password and Confirm Password do not match."));
            }
            let pairs = [
                ("action", String::from("add")),
                ("username", user),
                ("role", role),
                ("password", self.user_form.password.clone()),
            ];
            self.reset_user_form();
            Ok(encode_form(&pairs))
        }
    }

    fn build_wireless_payload(&self) -> Result<String, String> {
        if !self.is_admin() {
            return Err(String::from("Read-only Viewer account cannot modify Wireless settings."));
        }
        let ssid = self.wireless.ssid.trim();
        if ssid.is_empty() || ssid.len() > 32 {
            return Err(String::from("SSID must be between 1 and 32 characters."));
        }
        if self.wireless.auth_mode == "psk" {
            let psk_len = self.wireless.wpa_psk.len();
            if !(8..=64).contains(&psk_len) {
                return Err(String::from(
                    "WPA2 Passphrase must be between 8 and 63 characters.",
                ));
            }
        }
        let tx_pwr = self
            .wireless
            .tx_power
            .trim()
            .parse::<i32>()
            .map_err(|_| String::from("TX Power must be a number between 0 and 100."))?;
        if !(0..=100).contains(&tx_pwr) {
            return Err(String::from("TX Power must be between 0 and 100%."));
        }
        let kick = self
            .wireless
            .kick_rssi
            .trim()
            .parse::<i32>()
            .map_err(|_| String::from("Low-RSSI Kick Threshold must be 0 or -95 to -40 dBm."))?;
        if kick != 0 && !(-95..=-40).contains(&kick) {
            return Err(String::from(
                "Low-RSSI Kick Threshold must be 0 (Disabled) or between -95 and -40 dBm.",
            ));
        }
        let assoc = self
            .wireless
            .assoc_rssi
            .trim()
            .parse::<i32>()
            .map_err(|_| String::from("Min Association RSSI must be 0 or -95 to -40 dBm."))?;
        if assoc != 0 && !(-95..=-40).contains(&assoc) {
            return Err(String::from(
                "Min Association RSSI must be 0 (Disabled) or between -95 and -40 dBm.",
            ));
        }
        let probe_r = self
            .wireless
            .probe_rssi
            .trim()
            .parse::<i32>()
            .map_err(|_| String::from("Probe Response RSSI Threshold must be 0 or -95 to -50 dBm."))?;
        if probe_r != 0 && !(-95..=-50).contains(&probe_r) {
            return Err(String::from(
                "Probe Response RSSI Threshold must be 0 (Disabled) or between -95 and -50 dBm.",
            ));
        }
        let dtim = self
            .wireless
            .dtim
            .trim()
            .parse::<i32>()
            .map_err(|_| String::from("DTIM Interval must be between 1 and 255."))?;
        if !(1..=255).contains(&dtim) {
            return Err(String::from("DTIM Interval must be between 1 and 255."));
        }
        let mdid = self.wireless.ft_mdid.trim();
        let clean_mdid = if mdid.len() == 2 { mdid } else { "CC" };

        let mut pairs: Vec<(&str, String)> = vec![
            ("current_page", String::from("/index.asp")),
            ("next_page", String::new()),
            ("sid_list", String::from("WLANConfig11b;Storage;")),
            ("group_id", String::new()),
            ("action_mode", String::from(" Apply ")),
            ("action_script", String::new()),
            (
                "rt_radio_x",
                String::from(if self.wireless.radio_enable { "1" } else { "0" }),
            ),
            ("rt_ssid", ssid.to_string()),
            (
                "rt_closed",
                String::from(if self.wireless.hidden_ssid { "1" } else { "0" }),
            ),
            ("rt_auth_mode", self.wireless.auth_mode.clone()),
            ("rt_channel", self.wireless.channel.clone()),
            ("rt_HT_BW", self.wireless.ht_bw.clone()),
            ("rt_TxPower", tx_pwr.to_string()),
            (
                "rt_gmode",
                String::from(if self.wireless.pure_ofdm { "5" } else { "2" }),
            ),
            (
                "cc_cell_breathing",
                String::from(if self.wireless.cell_breathing { "1" } else { "0" }),
            ),
            (
                "cc_qload_enable",
                String::from(if self.wireless.qload_enable { "1" } else { "0" }),
            ),
            ("cc_probe_rssi", probe_r.to_string()),
            (
                "rt_rrm_enable",
                String::from(if self.wireless.rrm_80211k { "1" } else { "0" }),
            ),
            (
                "rt_ft_enable",
                String::from(if self.wireless.ft_80211r { "1" } else { "0" }),
            ),
            ("rt_ft_mdid", clean_mdid.to_string()),
            ("rt_KickStaRssiLow", kick.to_string()),
            ("rt_AssocReqRssiThres", assoc.to_string()),
            ("rt_dtim", dtim.to_string()),
            (
                "rt_ap_isolate",
                String::from(if self.wireless.ap_isolate { "1" } else { "0" }),
            ),
        ];

        if self.wireless.auth_mode == "psk" {
            pairs.push(("rt_wpa_mode", String::from("2")));
            pairs.push(("rt_crypto", String::from("aes")));
            pairs.push(("rt_wpa_psk", self.wireless.wpa_psk.clone()));
        }

        let g_vlan = if self.wireless.guest_vlan_id.trim().is_empty() {
            0
        } else {
            self.wireless
                .guest_vlan_id
                .trim()
                .parse::<i32>()
                .map_err(|_| String::from("Guest 802.1Q VLAN ID must be 0 (Untagged) or 2–4094."))?
        };
        if g_vlan != 0 && !(2..=4094).contains(&g_vlan) {
            return Err(String::from(
                "Guest 802.1Q VLAN ID must be 0 (Untagged) or between 2 and 4094.",
            ));
        }

        pairs.push((
            "rt_guest_enable",
            String::from(if self.wireless.guest_enable { "1" } else { "0" }),
        ));
        pairs.push(("cc_guest_vlan", g_vlan.to_string()));
        if self.wireless.guest_enable {
            let g_ssid = self.wireless.guest_ssid.trim();
            if g_ssid.is_empty() || g_ssid.len() > 32 {
                return Err(String::from("Secondary / Guest SSID must be between 1 and 32 characters."));
            }
            if self.wireless.guest_auth_mode == "psk" {
                let gpsk_len = self.wireless.guest_wpa_psk.len();
                if !(8..=64).contains(&gpsk_len) {
                    return Err(String::from(
                        "Secondary / Guest WPA2 Passphrase must be between 8 and 63 characters.",
                    ));
                }
                pairs.push(("rt_guest_auth_mode", String::from("psk")));
                pairs.push(("rt_guest_wpa_mode", String::from("2")));
                pairs.push(("rt_guest_crypto", String::from("aes")));
                pairs.push(("rt_guest_wpa_psk", self.wireless.guest_wpa_psk.clone()));
            } else {
                pairs.push(("rt_guest_auth_mode", String::from("open")));
                pairs.push(("rt_guest_wpa_mode", String::from("0")));
                pairs.push(("rt_guest_crypto", String::from("none")));
            }
            pairs.push(("rt_guest_ssid", g_ssid.to_string()));
            pairs.push((
                "rt_guest_lan_isolate",
                String::from(if self.wireless.guest_lan_isolate { "1" } else { "0" }),
            ));
            pairs.push((
                "rt_guest_ap_isolate",
                String::from(if self.wireless.guest_ap_isolate { "1" } else { "0" }),
            ));
        }

        Ok(encode_form(&pairs))
    }

    fn build_network_payload(&mut self) -> Result<(String, String), String> {
        if !self.is_admin() {
            return Err(String::from("Read-only Viewer account cannot modify Network settings."));
        }
        let host = self.network.hostname.trim().replace(' ', "-");
        if host.is_empty() || host.len() > 32 {
            return Err(String::from("Device Hostname must be 1 to 32 characters."));
        }
        if !host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return Err(String::from(
                "Hostname may only contain letters, digits, '.', '-', or '_'.",
            ));
        }
        let loc = self.network.device_loc.trim();
        if loc.len() > 64 {
            return Err(String::from("Device Location / Notes must be 64 characters or less."));
        }
        let ctrl_u = self.network.ctrl_url.trim();
        if !ctrl_u.is_empty()
            && !ctrl_u.starts_with("http://")
            && !ctrl_u.starts_with("https://")
        {
            return Err(String::from(
                "Controller URL must start with http:// or https:// (or leave blank).",
            ));
        }

        self.network.hostname = host.clone();
        self.live.hostname = host.clone();
        self.live.device_loc = loc.to_string();

        let mut redirect_ip = String::new();
        let mut pairs: Vec<(&str, String)> = vec![
            ("current_page", String::from("/index.asp")),
            ("next_page", String::new()),
            ("sid_list", String::from("LANHostConfig;Storage;")),
            ("group_id", String::new()),
            ("action_mode", String::from(" Apply ")),
            ("action_script", String::new()),
            ("computer_name", host),
            ("cc_device_loc", loc.to_string()),
            (
                "cc_fair_queue",
                String::from(if self.network.fair_queue { "1" } else { "0" }),
            ),
            (
                "cc_l2_shield",
                String::from(if self.network.l2_shield { "1" } else { "0" }),
            ),
            ("cc_ctrl_url", ctrl_u.to_string()),
            (
                "lan_proto_x",
                String::from(if self.network.dhcp_client { "1" } else { "0" }),
            ),
        ];

        if !self.network.dhcp_client {
            let ip = self.network.ipaddr.trim();
            let mask = self.network.netmask.trim();
            let gw = self.network.gateway.trim();
            if !is_valid_ipv4(ip) {
                return Err(String::from("Please enter a valid Static IPv4 Address."));
            }
            if !is_valid_ipv4(mask) {
                return Err(String::from("Please enter a valid Subnet Mask."));
            }
            if !is_valid_ipv4(gw) {
                return Err(String::from("Please enter a valid Default Gateway."));
            }
            let d1 = self.network.dns1.trim();
            let d2 = self.network.dns2.trim();
            if !d1.is_empty() && !is_valid_ipv4(d1) {
                return Err(String::from("Primary DNS is not a valid IPv4 address."));
            }
            if !d2.is_empty() && !is_valid_ipv4(d2) {
                return Err(String::from("Secondary DNS is not a valid IPv4 address."));
            }

            if ip != self.live.live_ip {
                redirect_ip = ip.to_string();
            }
            let manual_dns = !d1.is_empty() || !d2.is_empty();
            pairs.push(("lan_ipaddr", ip.to_string()));
            pairs.push(("lan_netmask", mask.to_string()));
            pairs.push(("lan_gateway", gw.to_string()));
            pairs.push((
                "lan_dns_x",
                String::from(if manual_dns { "0" } else { "1" }),
            ));
            pairs.push(("lan_dns1", d1.to_string()));
            pairs.push(("lan_dns2", d2.to_string()));
        }

        Ok((encode_form(&pairs), redirect_ip))
    }

    fn build_system_payload(&mut self) -> Result<String, String> {
        if !self.is_admin() {
            return Err(String::from("Read-only Viewer account cannot modify System settings."));
        }
        let ntp = self.system.ntp_server.trim();
        let led_m = if self.system.led_mode == "1" { "1" } else { "0" };
        let ota_u = if self.system.ota_repo_url.trim().is_empty() {
            DEFAULT_OTA_REPO
        } else {
            self.system.ota_repo_url.trim()
        };
        if !ota_u.starts_with("https://") && !ota_u.starts_with("http://") {
            return Err(String::from(
                "GitHub Repository URL must start with https:// or http://.",
            ));
        }

        let pairs: Vec<(&str, String)> = vec![
            ("current_page", String::from("/index.asp")),
            ("next_page", String::new()),
            ("sid_list", String::from("LANHostConfig;General;Storage;")),
            ("group_id", String::new()),
            ("action_mode", String::from(" Apply ")),
            ("action_script", String::new()),
            ("time_zone", self.system.time_zone.clone()),
            ("ntp_server0", ntp.to_string()),
            ("cc_led_mode", String::from(led_m)),
            ("cc_ota_repo_url", ota_u.to_string()),
        ];

        Ok(encode_form(&pairs))
    }

    fn render_footer(&self) -> &'static str {
        r#"<footer class="footer">
  <div class="footer-inner">
    Developed by <a href="https://viettechsoft.com" target="_blank" rel="noopener noreferrer">Viettechsoft</a>
  </div>
</footer>"#
    }

    fn render_login(&self) -> String {
        let disp_host = if !self.live.hostname.is_empty() {
            &self.live.hostname
        } else {
            &self.network.hostname
        };
        let host = html_escape(disp_host);
        let build_info = self.build_badge_str();
        let ip_part = if self.live.live_ip.is_empty() {
            String::new()
        } else {
            format!(" &bull; {}", html_escape(&self.live.live_ip))
        };
        let p_type = if self.login_show_pass {
            "text"
        } else {
            "password"
        };
        let p_lbl = if self.login_show_pass { "Hide" } else { "Show" };
        let btn_lbl = if self.login_busy {
            "Signing in..."
        } else {
            "Sign In to Access Point"
        };
        let dis = if self.login_busy { "disabled" } else { "" };

        format!(
            r#"<div class="login-shell">
  <div class="login-center">
    <div class="login-card">
      <div class="login-brand">
        <div class="brand-logo-lg" aria-hidden="true">
          <svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M5 12.55a11 11 0 0 1 14.08 0"/>
            <path d="M1.42 9a16 16 0 0 1 21.16 0"/>
            <path d="M8.53 16.11a6 6 0 0 1 6.95 0"/>
            <circle cx="12" cy="20" r="1"/>
          </svg>
        </div>
        <div>
          <div class="login-title">{ver} <span class="host-chip">{host}</span></div>
          <div class="login-sub">{build_info}{ip_part}</div>
        </div>
      </div>

      {notice}

      <form class="form-stack" data-action="login-submit" onsubmit="return false;">
        <div class="field">
          <label class="field-label">Username</label>
          <input type="text" class="input" maxlength="32" autocomplete="username" data-field="login_user" value="{user}" placeholder="Enter username" {dis}>
        </div>
        <div class="field">
          <label class="field-label">Password</label>
          <div class="input-group">
            <input type="{p_type}" class="input" maxlength="64" autocomplete="current-password" data-field="login_pass" value="{pass}" placeholder="Enter password" {dis}>
            <button type="button" class="btn btn-outline" data-action="toggle-login-pass" {dis}>{p_lbl}</button>
          </div>
        </div>
        <button type="submit" class="btn btn-primary btn-full" data-action="login" {dis}>{btn_lbl}</button>
      </form>
    </div>
  </div>
  {footer}
</div>"#,
            ver = CURRENT_VERSION_LABEL,
            host = host,
            build_info = build_info,
            ip_part = ip_part,
            notice = self.render_notice(),
            user = html_escape(&self.login_user),
            p_type = p_type,
            pass = html_escape(&self.login_pass),
            p_lbl = p_lbl,
            btn_lbl = btn_lbl,
            dis = dis,
            footer = self.render_footer(),
        )
    }

    fn render_locate_btn(&self) -> String {
        let dis = if self.is_admin() { "" } else { "disabled" };
        if self.live.locate_remaining_sec > 0 {
            format!(
                r#"<button type="button" class="btn btn-locate-active btn-sm" data-action="locate-ap" data-locate-active="1" title="Click to stop ceiling LED blink" {dis}>Locating LED <span class="locate-badge">{rem}s</span></button>"#,
                dis = dis,
                rem = self.live.locate_remaining_sec
            )
        } else if self.is_admin() {
            String::from(
                r#"<button type="button" class="btn btn-locate btn-sm" data-action="locate-ap" data-locate-active="0" title="Blink Blue/Red ceiling LED for 30 seconds">Locate LED (30s)</button>"#,
            )
        } else {
            String::new()
        }
    }

    fn render_header(&self) -> String {
        let ch_txt = if self.live.live_channel > 0 {
            format!("CH {}", self.live.live_channel)
        } else {
            String::from("2.4 GHz")
        };
        let cl_cnt = self.live.clients.len();
        let cl_txt = if cl_cnt == 1 {
            String::from("1 Client")
        } else {
            format!("{} Clients", cl_cnt)
        };
        let disp_host = if !self.live.hostname.is_empty() {
            &self.live.hostname
        } else {
            &self.network.hostname
        };
        let host = html_escape(disp_host);
        let build_badge = self.build_badge_str();
        let loc_sub = if self.live.device_loc.is_empty() {
            format!("Enterprise Access Point &bull; <span class=\"mono-xs\">{}</span>", build_badge)
        } else {
            format!(
                "{} &bull; <span class=\"mono-xs\">{}</span>",
                html_escape(&self.live.device_loc),
                build_badge
            )
        };

        let tab_btn = |t: Tab, label: &str| -> String {
            let active = if self.tab == t { " active" } else { "" };
            format!(
                r#"<button type="button" class="nav-tab{active}" data-action="tab" data-tab="{id}">{label}</button>"#,
                active = active,
                id = t.as_str(),
                label = label
            )
        };

        let cur_user = if self.auth_user.is_empty() {
            String::from("admin")
        } else {
            html_escape(&self.auth_user)
        };
        let (role_lbl, role_cls) = if self.is_admin() {
            ("Admin", "role-chip")
        } else {
            ("Viewer", "role-chip role-chip-viewer")
        };
        let locate_btn = self.render_locate_btn();
        let reboot_btn = if self.is_admin() {
            r#"<button type="button" class="btn btn-outline btn-sm" data-action="reboot">Reboot</button>"#
        } else {
            ""
        };

        format!(
            r#"<header class="topbar">
  <div class="topbar-inner">
    <div class="brand">
      <div class="brand-logo" aria-hidden="true">
        <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M5 12.55a11 11 0 0 1 14.08 0"/>
          <path d="M1.42 9a16 16 0 0 1 21.16 0"/>
          <path d="M8.53 16.11a6 6 0 0 1 6.95 0"/>
          <circle cx="12" cy="20" r="1"/>
        </svg>
      </div>
      <div class="brand-meta">
        <div class="brand-title">{ver} <span class="host-chip">{host}</span></div>
        <div class="brand-sub">{loc_sub}</div>
      </div>
    </div>

    <nav class="nav-tabs" aria-label="Primary Navigation">
      {t_dash}
      {t_wifi}
      {t_net}
      {t_users}
      {t_sys}
    </nav>

    <div class="topbar-actions">
      {locate_btn}
      <span class="live-pill"><span class="dot"></span>{ch_txt} &bull; {cl_txt}</span>
      <span class="user-pill"><strong>{cur_user}</strong><span class="{role_cls}">{role_lbl}</span></span>
      {reboot_btn}
      <button type="button" class="btn btn-outline btn-sm" data-action="logout">Logout</button>
    </div>
  </div>
</header>"#,
            ver = CURRENT_VERSION_LABEL,
            host = host,
            loc_sub = loc_sub,
            t_dash = tab_btn(Tab::Dashboard, "Dashboard"),
            t_wifi = tab_btn(Tab::Wireless, "Wireless"),
            t_net = tab_btn(Tab::Network, "Network"),
            t_users = tab_btn(Tab::Users, "Users"),
            t_sys = tab_btn(Tab::System, "System"),
            locate_btn = locate_btn,
            ch_txt = ch_txt,
            cl_txt = cl_txt,
            cur_user = cur_user,
            role_cls = role_cls,
            role_lbl = role_lbl,
            reboot_btn = reboot_btn,
        )
    }

    fn render_notice(&self) -> String {
        if self.notice_msg.is_empty() {
            return String::new();
        }
        let cls = match self.notice_kind.as_str() {
            "err" => "alert alert-err",
            "ok" => "alert alert-ok",
            _ => "alert alert-info",
        };
        format!(
            r#"<div class="{cls}"><span>{msg}</span><button type="button" class="alert-close" data-action="clear-notice">&times;</button></div>"#,
            cls = cls,
            msg = html_escape(&self.notice_msg)
        )
    }

    fn render_viewer_banner(&self) -> &'static str {
        if self.is_admin() {
            ""
        } else {
            r#"<div class="alert alert-info"><span>Signed in as <strong>Viewer (Read-Only)</strong>. Configuration changes require an Administrator account.</span></div>"#
        }
    }

    fn render_uplink_card(&self) -> String {
        /* TOTOLINK N9v1 has a single physical RJ45 LAN/PoE port.
         * Find whichever internal EPHY port is active (typically port 0) to report exact speed/duplex. */
        let active_port = self.live.ports.iter().find(|p| p.up);
        let speed = active_port
            .map(|p| p.speed)
            .unwrap_or(if self.live.link_up {
                self.live.max_speed_mbps.max(100)
            } else {
                0
            });
        let dup_raw = active_port
            .map(|p| p.duplex.as_str())
            .unwrap_or(if self.live.link_up { "FD" } else { "NONE" });
        let dup_txt = if dup_raw == "FD" {
            "Full-Duplex"
        } else if dup_raw == "HD" {
            "Half-Duplex"
        } else {
            "No Carrier"
        };

        let (box_cls, link_badge) = if !self.live.link_up || speed == 0 {
            (
                "uplink-banner uplink-banner-down",
                String::from(r#"<span class="badge badge-red">Link Down — Cable Unplugged</span>"#),
            )
        } else if self.live.degraded_10m || speed == 10 || dup_raw == "HD" {
            (
                "uplink-banner uplink-banner-warn",
                format!(
                    r#"<span class="badge badge-amber">{} Mbps {} — Degraded Cable Warning</span>"#,
                    speed, dup_txt
                ),
            )
        } else {
            (
                "uplink-banner uplink-banner-up",
                format!(
                    r#"<span class="badge badge-green">{} Mbps {}</span>"#,
                    speed, dup_txt
                ),
            )
        };

        let gw_disp = if self.live.live_gw.is_empty() {
            html_escape(&self.network.gateway)
        } else {
            html_escape(&self.live.live_gw)
        };
        let gw_badge = if self.live.gateway_reachable && self.live.link_up {
            format!(
                r#"<span class="badge badge-green">Gateway Reachable ({})</span>"#,
                gw_disp
            )
        } else {
            format!(
                r#"<span class="badge badge-red">Gateway Unreachable ({}) &bull; Ceiling LED Red</span>"#,
                gw_disp
            )
        };

        let rescue_badge = if self.live.fallback_alias_active {
            r#" <span class="badge badge-amber">Rescue Alias 192.168.1.165 Active</span>"#
        } else {
            ""
        };

        let fq_tag = if self.network.fair_queue {
            r#"<span class="tag tag-green">Per-Host FQ-CoDel</span>"#
        } else {
            r#"<span class="tag">Standard FQ-CoDel</span>"#
        };
        let shield_tag = if self.network.l2_shield {
            r#"<span class="tag tag-blue">L2 Broadcast &amp; Wi-Fi Admin Shield</span>"#
        } else {
            r#"<span class="tag">Rogue DHCP Guard Only</span>"#
        };
        let led_tag = if self.system.led_mode == "1" {
            r#"<span class="tag tag-blue">Ceiling LED: Bedroom Night Mode</span>"#
        } else {
            r#"<span class="tag">Ceiling LED: Online=Green / Offline=Red</span>"#
        };

        format!(
            r#"<section class="card port-strip-card">
  <div class="port-strip-head">
    <div class="port-strip-title">
      <span>Physical Ethernet Port &mdash; <strong>LAN / PoE Uplink</strong> (<code>br0</code>)</span>
      {link_badge}
      {gw_badge}{rescue_badge}
    </div>
    <div class="port-strip-meta">LAN RX: <strong>{lrx}</strong> &bull; TX: <strong>{ltx}</strong></div>
  </div>
  <div class="{box_cls}">
    <div class="uplink-port-left">
      <span class="port-name">RJ45 LAN / PoE (802.3af / Passive PoE)</span>
      <span class="sub-cell">Single physical uplink bridged to <code>br0</code> &bull; Active IPv4: <strong class="mono">{ip}</strong></span>
    </div>
    <div class="tag-row" style="margin-top:0">{fq_tag}{shield_tag}{led_tag}</div>
  </div>
</section>"#,
            link_badge = link_badge,
            gw_badge = gw_badge,
            rescue_badge = rescue_badge,
            lrx = format_bytes(self.live.lan_rx_bytes),
            ltx = format_bytes(self.live.lan_tx_bytes),
            box_cls = box_cls,
            ip = if self.live.live_ip.is_empty() {
                html_escape(&self.network.ipaddr)
            } else {
                html_escape(&self.live.live_ip)
            },
            fq_tag = fq_tag,
            shield_tag = shield_tag,
            led_tag = led_tag,
        )
    }

    fn render_dashboard(&self) -> String {
        let radio_badge = if self.wireless.radio_enable {
            r#"<span class="badge badge-green">Radio Enabled</span>"#
        } else {
            r#"<span class="badge badge-muted">Radio Disabled</span>"#
        };
        let sec_label = if self.wireless.auth_mode == "psk" {
            "WPA2-Personal (AES)"
        } else {
            "Open System (No Password)"
        };
        let k_badge = if self.wireless.rrm_80211k {
            r#"<span class="tag tag-blue">802.11k/v BTM</span>"#
        } else {
            r#"<span class="tag">802.11k Off</span>"#
        };
        let r_badge = if self.wireless.ft_80211r && self.wireless.auth_mode == "psk" {
            r#"<span class="tag tag-blue">802.11r FT</span>"#
        } else if self.wireless.auth_mode != "psk" {
            r#"<span class="tag tag-green">Open Fast-Auth</span>"#
        } else {
            r#"<span class="tag">802.11r Off</span>"#
        };
        let ofdm_badge = if self.wireless.pure_ofdm {
            r#"<span class="tag tag-green">Pure OFDM</span>"#
        } else {
            r#"<span class="tag">802.11b/g/n</span>"#
        };
        let qload_badge = if self.wireless.qload_enable {
            r#"<span class="tag tag-blue">802.11e QBSS Load</span>"#
        } else {
            ""
        };
        let g_badge = if self.wireless.guest_enable {
            if self.wireless.guest_vlan_id != "0" && !self.wireless.guest_vlan_id.is_empty() {
                format!(
                    r#"<span class="tag tag-green">+VAP VLAN {}</span>"#,
                    html_escape(&self.wireless.guest_vlan_id)
                )
            } else {
                String::from(r#"<span class="tag tag-green">+Guest VAP</span>"#)
            }
        } else {
            String::new()
        };

        let ch_mode = if self.wireless.channel == "0" {
            if self.live.live_channel > 0 {
                format!("Channel {} (Auto)", self.live.live_channel)
            } else {
                String::from("Auto Channel")
            }
        } else {
            format!("Channel {} (Fixed)", html_escape(&self.wireless.channel))
        };
        let bw_txt = if self.wireless.ht_bw == "1" {
            "20/40 MHz"
        } else {
            "20 MHz"
        };

        let cca_pct = self.live.channel_busy_pct.min(100);
        let cca_bar_cls = if cca_pct >= 75 {
            "progress-bar progress-bar-crit"
        } else if cca_pct >= 45 {
            "progress-bar progress-bar-warn"
        } else {
            "progress-bar progress-bar-ok"
        };
        let breathing_badge = if self.live.breathing_active {
            r#" &bull; <span class="tag tag-amber">Cell Breathing (-74dBm)</span>"#
        } else {
            ""
        };
        let probe_note = if self.wireless.probe_rssi != "0" && !self.wireless.probe_rssi.is_empty()
        {
            format!(" &bull; Probe &ge; {}dBm", html_escape(&self.wireless.probe_rssi))
        } else {
            String::new()
        };

        let ram_pct = if self.live.ram_total_kb > 0 {
            ((self.live.ram_used_kb * 100) / self.live.ram_total_kb).min(100)
        } else {
            10
        };
        let ram_used_mb = (self.live.ram_used_kb as f64) / 1024.0;
        let ram_tot_mb = (self.live.ram_total_kb as f64) / 1024.0;
        let build_code = if self.live.build_code.is_empty() {
            String::from(DEFAULT_BUILD_CODE)
        } else {
            html_escape(&self.live.build_code)
        };
        let ip_disp = if self.live.live_ip.is_empty() {
            html_escape(&self.network.ipaddr)
        } else {
            html_escape(&self.live.live_ip)
        };
        let mac_disp = if self.live.ap_mac.is_empty() {
            String::from("—")
        } else {
            html_escape(&self.live.ap_mac)
        };

        let mut rows_html = String::new();
        if self.live.clients.is_empty() {
            rows_html.push_str(
                r#"<tr><td colspan="8" class="empty-cell">No wireless stations are currently associated with this Access Point.</td></tr>"#,
            );
        } else {
            for cl in &self.live.clients {
                let (sig_cls, sig_lbl) = if cl.rssi_dbm >= -60 {
                    ("sig-good", "Excellent")
                } else if cl.rssi_dbm >= -72 {
                    ("sig-ok", "Good")
                } else if cl.rssi_dbm >= -80 {
                    ("sig-warn", "Fair")
                } else {
                    ("sig-bad", "Weak")
                };
                let host_note = if cl.hostname.is_empty() {
                    String::new()
                } else {
                    format!(r#"<div class="sub-cell">{}</div>"#, html_escape(&cl.hostname))
                };
                let os_cell = if cl.os_hint.is_empty() {
                    String::from(r#"<span class="tag">Standard Wi-Fi</span>"#)
                } else if cl.os_hint.contains("Apple") {
                    format!(
                        r#"<span class="tag tag-blue">{}</span>"#,
                        html_escape(&cl.os_hint)
                    )
                } else if cl.os_hint.contains("Android") || cl.os_hint.contains("Samsung") {
                    format!(
                        r#"<span class="tag tag-green">{}</span>"#,
                        html_escape(&cl.os_hint)
                    )
                } else {
                    format!(r#"<span class="tag">{}</span>"#, html_escape(&cl.os_hint))
                };
                let vap_cell = if cl.vap == "Guest" {
                    format!(
                        r#"<span class="tag tag-green">ra1</span> {}"#,
                        html_escape(&self.wireless.guest_ssid)
                    )
                } else {
                    format!(
                        r#"<span class="tag tag-blue">ra0</span> {}"#,
                        html_escape(&self.wireless.ssid)
                    )
                };
                let rx_disp = if cl.rx_rate_mbps.is_empty() {
                    html_escape(&cl.tx_rate_mbps)
                } else {
                    html_escape(&cl.rx_rate_mbps)
                };
                rows_html.push_str(&format!(
                    r#"<tr>
  <td class="mono">{mac}{host_note}</td>
  <td>{os_cell}</td>
  <td class="mono">{ip}</td>
  <td>{vap_cell}</td>
  <td>{mode} &bull; {bw}</td>
  <td><span class="rate-up">&uarr; {tx_rate}</span> / <span class="rate-dn">&darr; {rx_rate}</span> Mbps</td>
  <td><span class="sig-pill {sig_cls}">{dbm} dBm ({pct}% &bull; {sig_lbl})</span></td>
  <td class="mono">{time}</td>
</tr>"#,
                    mac = html_escape(&cl.mac),
                    host_note = host_note,
                    os_cell = os_cell,
                    ip = html_escape(&cl.ip),
                    vap_cell = vap_cell,
                    mode = html_escape(&cl.phy_mode),
                    bw = html_escape(&cl.bw),
                    tx_rate = html_escape(&cl.tx_rate_mbps),
                    rx_rate = rx_disp,
                    sig_cls = sig_cls,
                    dbm = cl.rssi_dbm,
                    pct = cl.signal_pct,
                    sig_lbl = sig_lbl,
                    time = html_escape(&cl.connect_time),
                ));
            }
        }

        let secondary_ssid_html = if self.wireless.guest_enable && self.wireless.radio_enable {
            let g_sec = if self.wireless.guest_auth_mode == "psk" {
                "WPA2-Personal (AES)"
            } else {
                "Open System (No Password)"
            };
            let g_vlan_txt = if self.wireless.guest_vlan_id != "0"
                && !self.wireless.guest_vlan_id.is_empty()
            {
                format!(
                    "802.1Q Tagged VLAN {} (br_guest)",
                    html_escape(&self.wireless.guest_vlan_id)
                )
            } else {
                String::from("Bridged Untagged (br0)")
            };
            let g_mac = if self.live.guest_mac.is_empty() {
                String::from("Active on ra1")
            } else {
                html_escape(&self.live.guest_mac)
            };
            let g_clients = self
                .live
                .clients
                .iter()
                .filter(|c| c.vap == "Guest")
                .count();
            let mut g_tags = String::new();
            if self.wireless.guest_lan_isolate {
                g_tags.push_str(r#"<span class="tag tag-blue">LAN Isolated</span>"#);
            }
            if self.wireless.guest_ap_isolate {
                g_tags.push_str(r#"<span class="tag">Client Isolation</span>"#);
            }
            format!(
                r#"<section class="card section-card">
  <div class="card-header">
    <div>
      <h2 class="card-title">Secondary / Guest Wireless (<code>ra1</code>)</h2>
      <p class="card-desc">Active virtual AP on MediaTek hardware radio sharing Channel {ch}</p>
    </div>
    <span class="badge badge-green">Secondary SSID Active</span>
  </div>
  <div class="lease-banner" style="margin-bottom:0">
    <div><span class="lease-lbl">Secondary SSID</span><strong>{g_ssid}</strong></div>
    <div><span class="lease-lbl">Security Mode</span><strong>{g_sec}</strong></div>
    <div><span class="lease-lbl">VLAN / Bridge</span><strong class="mono">{g_vlan}</strong></div>
    <div><span class="lease-lbl">BSSID &bull; Stations</span><strong class="mono">{g_mac} ({g_cnt} clients)</strong></div>
  </div>
  {tag_wrap}
</section>"#,
                ch = if self.live.live_channel > 0 {
                    self.live.live_channel.to_string()
                } else {
                    html_escape(&self.wireless.channel)
                },
                g_ssid = html_escape(&self.wireless.guest_ssid),
                g_sec = g_sec,
                g_vlan = g_vlan_txt,
                g_mac = g_mac,
                g_cnt = g_clients,
                tag_wrap = if g_tags.is_empty() {
                    String::new()
                } else {
                    format!(r#"<div class="tag-row" style="margin-top:10px">{}</div>"#, g_tags)
                }
            )
        } else {
            String::new()
        };

        let (rrm_badge_html, rrm_rows_html) = if !self.wireless.rrm_80211k {
            (
                String::from(r#"<span class="badge badge-muted">802.11k Disabled</span>"#),
                String::from(
                    r#"<tr><td colspan="6" class="empty-cell">IEEE 802.11k Neighbor Report is currently disabled in Wireless settings.</td></tr>"#,
                ),
            )
        } else if self.live.rrm_neighbors.is_empty() {
            (
                String::from(r#"<span class="badge badge-blue">802.11k Enabled &bull; 0 Neighbors</span>"#),
                String::from(
                    r#"<tr><td colspan="6" class="empty-cell">802.11k RRM is active on hardware radio <code>ra0</code>. Neighbor AP entries will appear automatically once ConCard AP Monitor synchronizes nearby Access Points.</td></tr>"#,
                ),
            )
        } else {
            let mut nrows = String::new();
            for (idx, nb) in self.live.rrm_neighbors.iter().enumerate() {
                let (s_cls, s_lbl) = if nb.rssi_dbm >= -65 {
                    ("sig-good", "Strong")
                } else if nb.rssi_dbm >= -76 {
                    ("sig-ok", "Good")
                } else {
                    ("sig-warn", "Edge")
                };
                let tag_disp = if nb.tag.is_empty() {
                    String::from("Neighbor AP")
                } else {
                    html_escape(&nb.tag)
                };
                nrows.push_str(&format!(
                    r#"<tr>
  <td class="mono">{num}</td>
  <td class="mono"><strong>{bssid}</strong></td>
  <td>{tag}</td>
  <td><span class="badge badge-blue">CH {ch}</span></td>
  <td><span class="sig-pill {s_cls}">{rssi} dBm ({s_lbl})</span></td>
  <td class="mono">RegClass 81 &bull; 802.11n HT</td>
</tr>"#,
                    num = idx + 1,
                    bssid = html_escape(&nb.bssid),
                    tag = tag_disp,
                    ch = nb.channel,
                    s_cls = s_cls,
                    rssi = nb.rssi_dbm,
                    s_lbl = s_lbl,
                ));
            }
            (
                format!(
                    r#"<span class="badge badge-green">Active &bull; {} Neighbors in Hardware Radio</span>"#,
                    self.live.rrm_neighbors.len()
                ),
                nrows,
            )
        };

        let uplink_card_html = self.render_uplink_card();

        format!(
            r#"<section class="grid-4">
  <div class="card kpi-card">
    <div class="kpi-head"><span>Wireless 2.4 GHz (ra0)</span>{radio_badge}</div>
    <div class="kpi-main">{ssid}</div>
    <div class="kpi-sub">{sec_label}</div>
    <div class="tag-row">{k_badge}{r_badge}{ofdm_badge}{qload_badge}{g_badge}</div>
  </div>

  <div class="card kpi-card">
    <div class="kpi-head"><span>RF Channel &amp; CCA Busy</span><span class="badge badge-blue">{bw_txt}</span></div>
    <div class="kpi-main">{ch_mode}</div>
    <div class="progress-track"><div class="{cca_bar_cls}" style="width:{cca_pct}%"></div></div>
    <div class="kpi-sub">CCA Busy: <strong>{cca_pct}%</strong> &bull; False CCA: <strong>{fcca}/s</strong> &bull; TX: <strong>{txp}%</strong>{probe_note}{breathing_badge}</div>
  </div>

  <div class="card kpi-card">
    <div class="kpi-head"><span>Connected Stations</span><span class="badge badge-slate">Max 64</span></div>
    <div class="kpi-main">{cl_cnt} <span class="kpi-unit">active</span></div>
    <div class="kpi-sub">WLAN RX: <strong>{rx}</strong> &bull; TX: <strong>{tx}</strong></div>
  </div>

  <div class="card kpi-card">
    <div class="kpi-head"><span>System &amp; Memory</span><span class="mono-xs">{build_code}</span></div>
    <div class="kpi-main">{up_d}d {up_h:02}h {up_m:02}m</div>
    <div class="progress-track"><div class="progress-bar" style="width:{ram_pct}%"></div></div>
    <div class="kpi-sub">RAM {ram_used:.1} / {ram_tot:.1} MB ({ram_pct}%) &bull; {ip_disp} ({mac_disp})</div>
  </div>
</section>

{uplink_card_html}

{secondary_ssid_html}

<section class="card section-card">
  <div class="card-header">
    <div>
      <h2 class="card-title">Connected Wireless Clients</h2>
      <p class="card-desc">Live 802.11 stations on <code>ra0</code> / <code>ra1</code> with Zero-CPU Assoc IE + DHCP OS Fingerprinting</p>
    </div>
    <button type="button" class="btn btn-outline btn-sm" data-action="refresh">Refresh Now</button>
  </div>
  <div class="table-wrap">
    <table class="data-table">
      <thead>
        <tr>
          <th>MAC / Hostname</th>
          <th>Device / OS</th>
          <th>IPv4 Address</th>
          <th>SSID / VAP</th>
          <th>PHY / Width</th>
          <th>TX / RX Rate</th>
          <th>Signal (RSSI)</th>
          <th>Connected</th>
        </tr>
      </thead>
      <tbody>{rows_html}</tbody>
    </table>
  </div>
</section>

<section class="card section-card">
  <div class="card-header">
    <div>
      <h2 class="card-title">IEEE 802.11k Neighbor Report (RRM)</h2>
      <p class="card-desc">Zero-scan neighbor Access Point channels advertised in hardware <code>RrmNrList</code> for fast client roaming</p>
    </div>
    {rrm_badge_html}
  </div>
  <div class="table-wrap">
    <table class="data-table">
      <thead>
        <tr>
          <th>#</th>
          <th>Neighbor BSSID</th>
          <th>AP Name / Tag</th>
          <th>Channel</th>
          <th>Est. Signal</th>
          <th>RRM Info</th>
        </tr>
      </thead>
      <tbody>{rrm_rows_html}</tbody>
    </table>
  </div>
</section>"#,
            radio_badge = radio_badge,
            ssid = html_escape(&self.wireless.ssid),
            sec_label = sec_label,
            k_badge = k_badge,
            r_badge = r_badge,
            ofdm_badge = ofdm_badge,
            qload_badge = qload_badge,
            g_badge = g_badge,
            bw_txt = bw_txt,
            ch_mode = ch_mode,
            cca_bar_cls = cca_bar_cls,
            cca_pct = cca_pct,
            fcca = self.live.false_cca_1s,
            txp = html_escape(&self.wireless.tx_power),
            probe_note = probe_note,
            breathing_badge = breathing_badge,
            cl_cnt = self.live.clients.len(),
            rx = format_bytes(self.live.wlan_rx_bytes),
            tx = format_bytes(self.live.wlan_tx_bytes),
            build_code = build_code,
            up_d = self.live.uptime_days,
            up_h = self.live.uptime_hours,
            up_m = self.live.uptime_mins,
            ram_pct = ram_pct,
            ram_used = ram_used_mb,
            ram_tot = ram_tot_mb,
            ip_disp = ip_disp,
            mac_disp = mac_disp,
            uplink_card_html = uplink_card_html,
            secondary_ssid_html = secondary_ssid_html,
            rows_html = rows_html,
            rrm_badge_html = rrm_badge_html,
            rrm_rows_html = rrm_rows_html,
        )
    }

    fn render_wireless(&self) -> String {
        let ro = if self.is_admin() { "" } else { "disabled" };
        let mut ch_opts = String::new();
        let ch_list = [
            ("0", "Auto (Smart Channel)"),
            ("1", "Channel 1 (2412 MHz - Recommended)"),
            ("2", "Channel 2 (2417 MHz)"),
            ("3", "Channel 3 (2422 MHz)"),
            ("4", "Channel 4 (2427 MHz)"),
            ("5", "Channel 5 (2432 MHz)"),
            ("6", "Channel 6 (2437 MHz - Recommended)"),
            ("7", "Channel 7 (2442 MHz)"),
            ("8", "Channel 8 (2447 MHz)"),
            ("9", "Channel 9 (2452 MHz)"),
            ("10", "Channel 10 (2457 MHz)"),
            ("11", "Channel 11 (2462 MHz - Recommended)"),
            ("12", "Channel 12 (2467 MHz)"),
            ("13", "Channel 13 (2472 MHz)"),
        ];
        for (val, label) in ch_list {
            let sel = if self.wireless.channel == val {
                " selected"
            } else {
                ""
            };
            ch_opts.push_str(&format!(
                r#"<option value="{val}"{sel}>{label}</option>"#,
                val = val,
                sel = sel,
                label = label
            ));
        }

        let psk_section = if self.wireless.auth_mode == "psk" {
            let inp_type = if self.wireless.show_psk {
                "text"
            } else {
                "password"
            };
            let btn_lbl = if self.wireless.show_psk {
                "Hide"
            } else {
                "Show"
            };
            format!(
                r#"<div class="field">
  <label class="field-label">WPA2 Passphrase (8–63 characters)</label>
  <div class="input-group">
    <input type="{inp_type}" class="input" maxlength="64" data-field="w_psk" value="{psk}" placeholder="Enter Wi-Fi password" {ro}>
    <button type="button" class="btn btn-outline" data-action="toggle-psk">{btn_lbl}</button>
  </div>
</div>"#,
                inp_type = inp_type,
                psk = html_escape(&self.wireless.wpa_psk),
                btn_lbl = btn_lbl,
                ro = ro
            )
        } else {
            String::from(
                r#"<div class="hint-box">Open System is active (no Wi-Fi password). 4-Way EAPOL handshake is bypassed for instant ~15 ms L2 association, while <strong>802.11k Neighbor Report</strong>, <strong>802.11v BTM</strong>, and <strong>802.11e QBSS Load</strong> remain active.</div>"#,
            )
        };

        let guest_psk_section = if self.wireless.guest_auth_mode == "psk" {
            let ginp = if self.wireless.guest_show_psk { "text" } else { "password" };
            let gbl = if self.wireless.guest_show_psk { "Hide" } else { "Show" };
            let gdis = if !self.wireless.guest_enable || !self.is_admin() { "disabled" } else { "" };
            format!(
                r#"<div class="field">
  <label class="field-label">Secondary WPA2 Passphrase (8–63 chars)</label>
  <div class="input-group">
    <input type="{ginp}" class="input" maxlength="64" data-field="wg_psk" value="{gpsk}" placeholder="Enter Guest/Staff Wi-Fi password" {gdis}>
    <button type="button" class="btn btn-outline" data-action="toggle-guest-psk" {gdis}>{gbl}</button>
  </div>
</div>"#,
                ginp = ginp,
                gpsk = html_escape(&self.wireless.guest_wpa_psk),
                gbl = gbl,
                gdis = gdis
            )
        } else {
            String::new()
        };
        let gdis = if !self.wireless.guest_enable || !self.is_admin() {
            "disabled"
        } else {
            ""
        };

        format!(
            r#"{vbanner}<div class="grid-2">
  <section class="card section-card">
    <div class="card-header">
      <div>
        <h2 class="card-title">Wireless SSID &amp; Security</h2>
        <p class="card-desc">Primary 2.4 GHz Access Point network identity and authentication</p>
      </div>
    </div>

    <div class="form-stack">
      <label class="switch-row">
        <span>
          <strong>Enable 2.4 GHz Radio</strong>
          <small>Broadcast wireless service on interface <code>ra0</code></small>
        </span>
        <input type="checkbox" data-field="w_radio" {radio_chk} {ro}>
      </label>

      <div class="field">
        <label class="field-label">Network Name (SSID)</label>
        <input type="text" class="input" maxlength="32" data-field="w_ssid" value="{ssid}" {ro}>
      </div>

      <div class="field">
        <label class="field-label">Security Mode</label>
        <select class="input" data-field="w_auth" data-rerender="1" {ro}>
          <option value="open"{sel_open}>Open System (No Password)</option>
          <option value="psk"{sel_psk}>WPA2-Personal (AES)</option>
        </select>
      </div>

      {psk_section}

      <label class="switch-row">
        <span>
          <strong>Hide SSID</strong>
          <small>Do not broadcast SSID in beacon frames</small>
        </span>
        <input type="checkbox" data-field="w_hidden" {hid_chk} {ro}>
      </label>

      <label class="switch-row">
        <span>
          <strong>Client Isolation (AP Isolation)</strong>
          <small>Block direct Layer-2 traffic between wireless clients on this AP</small>
        </span>
        <input type="checkbox" data-field="w_isolate" {iso_chk} {ro}>
      </label>
    </div>
  </section>

  <section class="card section-card">
    <div class="card-header">
      <div>
        <h2 class="card-title">RF Channel, Airtime &amp; Fast Roaming</h2>
        <p class="card-desc">High-density channel tuning, QBSS load beacons, probe filtering, and 802.11k/v/r roaming</p>
      </div>
    </div>

    <div class="form-stack">
      <div class="grid-2-tight">
        <div class="field">
          <label class="field-label">Operating Channel</label>
          <select class="input" data-field="w_channel" {ro}>{ch_opts}</select>
        </div>
        <div class="field">
          <label class="field-label">Channel Bandwidth</label>
          <select class="input" data-field="w_bw" {ro}>
            <option value="0"{bw20}>20 MHz (High-Density)</option>
            <option value="1"{bw40}>20/40 MHz</option>
          </select>
        </div>
      </div>

      <div class="grid-2-tight">
        <div class="field">
          <label class="field-label">Transmit Power (0–100%)</label>
          <input type="number" class="input" min="0" max="100" data-field="w_txpower" value="{txp}" {ro}>
        </div>
        <div class="field">
          <label class="field-label">DTIM Interval (1–255)</label>
          <input type="number" class="input" min="1" max="255" data-field="w_dtim" value="{dtim}" {ro}>
        </div>
      </div>

      <div class="grid-3-tight">
        <div class="field">
          <label class="field-label">Low-RSSI Kick (dBm, 0=Off)</label>
          <input type="number" class="input" min="-95" max="0" data-field="w_kick" value="{kick}" {ro}>
        </div>
        <div class="field">
          <label class="field-label">Min Assoc RSSI (dBm, 0=Off)</label>
          <input type="number" class="input" min="-95" max="0" data-field="w_assoc" value="{assoc}" {ro}>
        </div>
        <div class="field">
          <label class="field-label">Probe Rsp RSSI (dBm, 0=Off)</label>
          <input type="number" class="input" min="-95" max="0" data-field="w_probe_rssi" value="{probe_rssi}" {ro}>
        </div>
      </div>

      <label class="switch-row">
        <span>
          <strong>Pure OFDM Mode (Disable 802.11b &mdash; <code>rt_gmode=5</code>)</strong>
          <small>Pure 802.11g/n OFDM BasicRate (6/12/24 Mbps) &mdash; eliminates legacy 802.11b CCK airtime drag</small>
        </span>
        <input type="checkbox" data-field="w_ofdm" {ofdm_chk} {ro}>
      </label>

      <label class="switch-row">
        <span>
          <strong>IEEE 802.11e QBSS Load Advertisement (<code>cc_qload_enable</code>)</strong>
          <small>Broadcasts real-time station count &amp; channel utilization (Tag 11) in beacons for smart AP selection</small>
        </span>
        <input type="checkbox" data-field="w_qload" {qload_chk} {ro}>
      </label>

      <label class="switch-row">
        <span>
          <strong>Adaptive Cell Breathing (<code>cc_cell_breathing</code>)</strong>
          <small>Automatically tighten association threshold (-74 dBm) under heavy load (&ge; 38 clients)</small>
        </span>
        <input type="checkbox" data-field="w_breathing" {breath_chk} {ro}>
      </label>

      <label class="switch-row">
        <span>
          <strong>IEEE 802.11k/v Roaming (RRM &amp; BTM)</strong>
          <small>Hardware Neighbor Report + 802.11v BSS Transition Management</small>
        </span>
        <input type="checkbox" data-field="w_rrm" {rrm_chk} {ro}>
      </label>

      <label class="switch-row">
        <span>
          <strong>IEEE 802.11r Fast BSS Transition (FT)</strong>
          <small>Accelerates WPA2-AES roaming across APs sharing the same Mobility Domain</small>
        </span>
        <input type="checkbox" data-field="w_ft" {ft_chk} {ro}>
      </label>

      <div class="field">
        <label class="field-label">Mobility Domain ID (2 chars)</label>
        <input type="text" class="input mono" maxlength="2" data-field="w_mdid" value="{mdid}" {ro}>
      </div>
    </div>
  </section>
</div>

<section class="card section-card" style="margin-top:16px">
  <div class="card-header">
    <div>
      <h2 class="card-title">Secondary / Guest SSID (<code>ra1</code>) &amp; 802.1Q VLAN</h2>
      <p class="card-desc">Optional second virtual AP on MediaTek hardware radio with optional 802.1Q VLAN tagging</p>
    </div>
    <span class="badge badge-blue">Hybrid Dual-VAP</span>
  </div>

  <div class="form-stack">
    <label class="switch-row">
      <span>
        <strong>Enable Secondary / Guest SSID (<code>ra1</code>)</strong>
        <small>Broadcast an independent second wireless network on the same 2.4 GHz channel</small>
      </span>
      <input type="checkbox" data-field="wg_enable" data-rerender="1" {g_en_chk} {ro}>
    </label>

    <div class="grid-3-tight">
      <div class="field">
        <label class="field-label">Secondary SSID Name</label>
        <input type="text" class="input" maxlength="32" data-field="wg_ssid" value="{g_ssid}" {gdis}>
      </div>
      <div class="field">
        <label class="field-label">Security Mode</label>
        <select class="input" data-field="wg_auth" data-rerender="1" {gdis}>
          <option value="open"{g_sel_open}>Open System (No Password)</option>
          <option value="psk"{g_sel_psk}>WPA2-Personal (AES)</option>
        </select>
      </div>
      <div class="field">
        <label class="field-label">802.1Q VLAN ID (0 = Untagged)</label>
        <input type="number" class="input mono" min="0" max="4094" data-field="wg_vlan" value="{g_vlan}" placeholder="0" {gdis}>
      </div>
    </div>

    {guest_psk_section}

    <div class="grid-2-tight">
      <label class="switch-row">
        <span>
          <strong>Isolate from Management LAN</strong>
          <small>Block access from Secondary SSID to private management subnet</small>
        </span>
        <input type="checkbox" data-field="wg_lan_iso" {g_lan_iso_chk} {gdis}>
      </label>
      <label class="switch-row">
        <span>
          <strong>Secondary Client Isolation</strong>
          <small>Block peer-to-peer L2 traffic between clients on <code>ra1</code></small>
        </span>
        <input type="checkbox" data-field="wg_ap_iso" {g_ap_iso_chk} {gdis}>
      </label>
    </div>
  </div>
</section>

<div class="action-bar">
  <button type="button" class="btn btn-primary" data-action="save-wireless" {ro}>Save &amp; Apply Wireless</button>
</div>"#,
            vbanner = self.render_viewer_banner(),
            ro = ro,
            radio_chk = if self.wireless.radio_enable { "checked" } else { "" },
            ssid = html_escape(&self.wireless.ssid),
            sel_open = if self.wireless.auth_mode != "psk" { " selected" } else { "" },
            sel_psk = if self.wireless.auth_mode == "psk" { " selected" } else { "" },
            psk_section = psk_section,
            hid_chk = if self.wireless.hidden_ssid { "checked" } else { "" },
            iso_chk = if self.wireless.ap_isolate { "checked" } else { "" },
            ch_opts = ch_opts,
            bw20 = if self.wireless.ht_bw != "1" { " selected" } else { "" },
            bw40 = if self.wireless.ht_bw == "1" { " selected" } else { "" },
            txp = html_escape(&self.wireless.tx_power),
            dtim = html_escape(&self.wireless.dtim),
            kick = html_escape(&self.wireless.kick_rssi),
            assoc = html_escape(&self.wireless.assoc_rssi),
            probe_rssi = html_escape(&self.wireless.probe_rssi),
            ofdm_chk = if self.wireless.pure_ofdm { "checked" } else { "" },
            qload_chk = if self.wireless.qload_enable { "checked" } else { "" },
            breath_chk = if self.wireless.cell_breathing { "checked" } else { "" },
            rrm_chk = if self.wireless.rrm_80211k { "checked" } else { "" },
            ft_chk = if self.wireless.ft_80211r { "checked" } else { "" },
            mdid = html_escape(&self.wireless.ft_mdid),
            g_en_chk = if self.wireless.guest_enable { "checked" } else { "" },
            gdis = gdis,
            g_ssid = html_escape(&self.wireless.guest_ssid),
            g_sel_open = if self.wireless.guest_auth_mode != "psk" { " selected" } else { "" },
            g_sel_psk = if self.wireless.guest_auth_mode == "psk" { " selected" } else { "" },
            g_vlan = html_escape(&self.wireless.guest_vlan_id),
            guest_psk_section = guest_psk_section,
            g_lan_iso_chk = if self.wireless.guest_lan_isolate { "checked" } else { "" },
            g_ap_iso_chk = if self.wireless.guest_ap_isolate { "checked" } else { "" },
        )
    }

    fn render_network(&self) -> String {
        let ro = if self.is_admin() { "" } else { "disabled" };
        let dis = if self.network.dhcp_client || !self.is_admin() {
            "disabled"
        } else {
            ""
        };
        let gw_status_txt = if self.live.gateway_reachable && self.live.link_up {
            "Reachable"
        } else {
            "Unreachable"
        };
        format!(
            r#"{vbanner}<section class="card section-card">
  <div class="card-header">
    <div>
      <h2 class="card-title">LAN &amp; IPv4 Management</h2>
      <p class="card-desc">Single physical LAN / PoE uplink bridge (<code>br0</code>) addressing, rescue alias, and device identity</p>
    </div>
    <span class="badge badge-blue">AP Bridge Mode</span>
  </div>

  <div class="lease-banner">
    <div><span class="lease-lbl">Active IPv4</span><strong class="mono">{live_ip}</strong></div>
    <div><span class="lease-lbl">Subnet Mask</span><strong class="mono">{live_mask}</strong></div>
    <div><span class="lease-lbl">Default Gateway ({gw_st})</span><strong class="mono">{live_gw}</strong></div>
    <div><span class="lease-lbl">MAC Address</span><strong class="mono">{mac}</strong></div>
  </div>

  <div class="form-stack">
    <div class="grid-3-tight">
      <div class="field">
        <label class="field-label">Device Hostname (ID)</label>
        <input type="text" class="input mono" maxlength="32" data-field="n_hostname" value="{host}" placeholder="e.g. SERVER, AP-305" {ro}>
      </div>
      <div class="field">
        <label class="field-label">Location / Description</label>
        <input type="text" class="input" maxlength="64" data-field="n_loc" value="{loc}" placeholder="e.g. Floor 3 - Room 305 / Server Room" {ro}>
      </div>
      <div class="field">
        <label class="field-label">IPv4 Assignment Mode</label>
        <select class="input" data-field="n_dhcp" data-rerender="1" {ro}>
          <option value="1"{sel_dhcp}>Automatic IP (DHCP + 192.168.1.165 Rescue Alias)</option>
          <option value="0"{sel_stat}>Static IPv4 Address</option>
        </select>
      </div>
    </div>

    <div class="grid-3-tight">
      <div class="field">
        <label class="field-label">IPv4 Address</label>
        <input type="text" class="input mono" maxlength="15" data-field="n_ip" value="{ip}" {dis}>
      </div>
      <div class="field">
        <label class="field-label">Subnet Mask</label>
        <input type="text" class="input mono" maxlength="15" data-field="n_mask" value="{mask}" {dis}>
      </div>
      <div class="field">
        <label class="field-label">Default Gateway</label>
        <input type="text" class="input mono" maxlength="15" data-field="n_gw" value="{gw}" {dis}>
      </div>
    </div>

    <div class="grid-2-tight">
      <div class="field">
        <label class="field-label">Primary DNS Server</label>
        <input type="text" class="input mono" maxlength="15" data-field="n_dns1" value="{dns1}" placeholder="Optional" {dis}>
      </div>
      <div class="field">
        <label class="field-label">Secondary DNS Server</label>
        <input type="text" class="input mono" maxlength="15" data-field="n_dns2" value="{dns2}" placeholder="Optional" {dis}>
      </div>
    </div>
  </div>
</section>

<section class="card section-card" style="margin-top:16px">
  <div class="card-header">
    <div>
      <h2 class="card-title">Traffic Fairness, L2 Security &amp; Controller Discovery</h2>
      <p class="card-desc">Hardware-friendly per-host queueing, broadcast storm suppression, Wi-Fi admin stealth, and heartbeat</p>
    </div>
    <span class="badge badge-green">v2.1 Protection</span>
  </div>

  <div class="form-stack">
    <div class="grid-2-tight">
      <label class="switch-row">
        <span>
          <strong>Per-Host Fair Queueing (<code>cc_fair_queue</code>)</strong>
          <small>FQ-CoDel + <code>cls_flow</code> per-IP hash &mdash; prevents multi-connection torrents from starving calls</small>
        </span>
        <input type="checkbox" data-field="n_fair_q" {fq_chk} {ro}>
      </label>

      <label class="switch-row">
        <span>
          <strong>L2 Broadcast &amp; Wi-Fi Admin Shield (<code>cc_l2_shield</code>)</strong>
          <small>Drops NetBIOS/SSDP/LLMNR broadcast noise and blocks SSH/HTTP/HTTPS access from Wi-Fi clients</small>
        </span>
        <input type="checkbox" data-field="n_l2_shield" {sh_chk} {ro}>
      </label>
    </div>

    <div class="field">
      <label class="field-label">Central Controller URL (<code>cc_ctrl_url</code> &mdash; Auto-Discovered or Custom)</label>
      <input type="text" class="input mono" maxlength="160" data-field="n_ctrl_url" value="{ctrl_url}" placeholder="http://192.168.1.102:9100" {ro}>
    </div>
  </div>

  <div class="action-bar">
    <button type="button" class="btn btn-primary" data-action="save-network" {ro}>Save &amp; Apply Network</button>
  </div>
</section>"#,
            vbanner = self.render_viewer_banner(),
            ro = ro,
            live_ip = html_escape(if self.live.live_ip.is_empty() { "—" } else { &self.live.live_ip }),
            live_mask = html_escape(if self.live.live_mask.is_empty() { "—" } else { &self.live.live_mask }),
            gw_st = gw_status_txt,
            live_gw = html_escape(if self.live.live_gw.is_empty() { "—" } else { &self.live.live_gw }),
            mac = html_escape(if self.live.ap_mac.is_empty() { "—" } else { &self.live.ap_mac }),
            host = html_escape(&self.network.hostname),
            loc = html_escape(&self.network.device_loc),
            sel_dhcp = if self.network.dhcp_client { " selected" } else { "" },
            sel_stat = if !self.network.dhcp_client { " selected" } else { "" },
            ip = html_escape(&self.network.ipaddr),
            mask = html_escape(&self.network.netmask),
            gw = html_escape(&self.network.gateway),
            dns1 = html_escape(&self.network.dns1),
            dns2 = html_escape(&self.network.dns2),
            dis = dis,
            fq_chk = if self.network.fair_queue { "checked" } else { "" },
            sh_chk = if self.network.l2_shield { "checked" } else { "" },
            ctrl_url = html_escape(&self.network.ctrl_url),
        )
    }

    fn render_users(&self) -> String {
        let p_type = if self.user_form.show_password {
            "text"
        } else {
            "password"
        };
        let p_lbl = if self.user_form.show_password {
            "Hide"
        } else {
            "Show"
        };

        if !self.is_admin() {
            return format!(
                r#"<div class="grid-2">
  <section class="card section-card">
    <div class="card-header">
      <div>
        <h2 class="card-title">My Account</h2>
        <p class="card-desc">Active session identity and access permissions</p>
      </div>
      <span class="badge badge-slate">Viewer (Read-Only)</span>
    </div>
    <div class="form-stack">
      <div class="field">
        <label class="field-label">Signed-In Username</label>
        <input type="text" class="input mono" value="{me}" disabled>
      </div>
      <div class="hint-box">
        <strong>Viewer</strong> accounts can monitor live 802.11 stations, RF status, and network parameters without altering AP configuration or firmware. Contact an Administrator to add or modify accounts.
      </div>
    </div>
  </section>

  <section class="card section-card">
    <div class="card-header">
      <div>
        <h2 class="card-title">Change My Password</h2>
        <p class="card-desc">Update the password for your account (<code>{me}</code>)</p>
      </div>
    </div>
    <div class="form-stack">
      <div class="field">
        <label class="field-label">Current Password</label>
        <input type="{p_type}" class="input" maxlength="64" data-field="um_old_pass" value="{old_p}" placeholder="Enter current password">
      </div>
      <div class="grid-2-tight">
        <div class="field">
          <label class="field-label">New Password (4–64 chars)</label>
          <input type="{p_type}" class="input" maxlength="64" data-field="um_pass" value="{p1}" placeholder="Enter new password">
        </div>
        <div class="field">
          <label class="field-label">Confirm New Password</label>
          <input type="{p_type}" class="input" maxlength="64" data-field="um_pass2" value="{p2}" placeholder="Re-enter new password">
        </div>
      </div>
      <div class="btn-row">
        <button type="button" class="btn btn-primary" data-action="save-user">Update My Password</button>
        <button type="button" class="btn btn-outline" data-action="toggle-um-pass">{p_lbl} Passwords</button>
      </div>
    </div>
  </section>
</div>"#,
                me = html_escape(&self.auth_user),
                p_type = p_type,
                old_p = html_escape(&self.user_form.old_password),
                p1 = html_escape(&self.user_form.password),
                p2 = html_escape(&self.user_form.confirm_password),
                p_lbl = p_lbl,
            );
        }

        let mut rows_html = String::new();
        if self.users.is_empty() {
            let def_u = html_escape(&self.primary_user);
            rows_html.push_str(&format!(
                r#"<tr>
  <td class="mono"><strong>{def_u}</strong> <span class="tag tag-blue">Primary</span></td>
  <td><span class="badge badge-blue">Administrator</span></td>
  <td><button type="button" class="btn btn-outline btn-xs" data-action="refresh-users">Reload</button></td>
</tr>"#
            ));
        } else {
            let admin_cnt = self.users.iter().filter(|u| u.role == "admin").count();
            for u in &self.users {
                let is_me = u.username == self.auth_user;
                let mut tags = String::new();
                if u.is_primary {
                    tags.push_str(r#" <span class="tag tag-blue">Primary</span>"#);
                }
                if is_me {
                    tags.push_str(r#" <span class="tag tag-green">You</span>"#);
                }
                let role_badge = if u.role == "admin" {
                    r#"<span class="badge badge-blue">Administrator</span>"#
                } else {
                    r#"<span class="badge badge-slate">Viewer (Read-Only)</span>"#
                };
                let can_delete = !u.is_primary && !is_me && !(u.role == "admin" && admin_cnt <= 1);
                let del_btn = if can_delete {
                    format!(
                        r#"<button type="button" class="btn btn-danger-outline btn-xs" data-action="delete-user" data-username="{u_esc}">Delete</button>"#,
                        u_esc = html_escape(&u.username)
                    )
                } else {
                    String::from(
                        r#"<button type="button" class="btn btn-outline btn-xs" disabled title="Protected account">Delete</button>"#,
                    )
                };
                rows_html.push_str(&format!(
                    r#"<tr>
  <td class="mono"><strong>{u_esc}</strong>{tags}</td>
  <td>{role_badge}</td>
  <td>
    <div class="table-actions">
      <button type="button" class="btn btn-outline btn-xs" data-action="edit-user" data-username="{u_esc}">Edit</button>
      {del_btn}
    </div>
  </td>
</tr>"#,
                    u_esc = html_escape(&u.username),
                    tags = tags,
                    role_badge = role_badge,
                    del_btn = del_btn,
                ));
            }
        }

        let (form_title, form_desc, submit_lbl) = if self.user_form.edit_mode {
            (
                format!("Edit User: {}", html_escape(&self.user_form.username)),
                "Update user role or set a new password",
                "Update User Account",
            )
        } else {
            (
                String::from("Add New User"),
                "Create an Administrator or Read-Only Viewer account",
                "Create User Account",
            )
        };

        let user_readonly = if self.user_form.edit_mode {
            "readonly"
        } else {
            ""
        };
        let role_dis = if self.user_form.edit_mode && self.user_form.edit_is_primary {
            "disabled"
        } else {
            ""
        };
        let pass_ph = if self.user_form.edit_mode {
            "Leave blank to keep unchanged"
        } else {
            "4–64 characters"
        };
        let cancel_btn = if self.user_form.edit_mode {
            r#"<button type="button" class="btn btn-outline" data-action="reset-user-form">Cancel / + New User</button>"#
        } else {
            ""
        };

        format!(
            r#"<div class="grid-2">
  <section class="card section-card">
    <div class="card-header">
      <div>
        <h2 class="card-title">User Accounts ({cnt} / 16)</h2>
        <p class="card-desc">Persistent multi-user access control stored in SPI flash (<code>/etc/storage</code>)</p>
      </div>
      <button type="button" class="btn btn-outline btn-sm" data-action="refresh-users">Refresh</button>
    </div>

    <div class="table-wrap">
      <table class="data-table">
        <thead>
          <tr>
            <th>Username</th>
            <th>Access Role</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>{rows_html}</tbody>
      </table>
    </div>

    <div class="hint-box" style="margin-top:14px">
      Updating the <strong>Primary</strong> account password automatically synchronizes SSH system login and ConCard AP Monitor HTTP credentials.
    </div>
  </section>

  <section class="card section-card">
    <div class="card-header">
      <div>
        <h2 class="card-title">{form_title}</h2>
        <p class="card-desc">{form_desc}</p>
      </div>
    </div>

    <div class="form-stack">
      <div class="field">
        <label class="field-label">Username</label>
        <input type="text" class="input mono" maxlength="32" data-field="um_username" value="{u_val}" placeholder="e.g. operator1" {user_readonly}>
      </div>

      <div class="field">
        <label class="field-label">Access Role</label>
        <select class="input" data-field="um_role" {role_dis}>
          <option value="admin"{sel_admin}>Administrator — Full Read/Write &amp; Firmware Access</option>
          <option value="viewer"{sel_viewer}>Viewer — Read-Only Monitoring Access</option>
        </select>
      </div>

      <div class="field">
        <label class="field-label">Password</label>
        <div class="input-group">
          <input type="{p_type}" class="input" maxlength="64" data-field="um_pass" value="{p1}" placeholder="{pass_ph}">
          <button type="button" class="btn btn-outline" data-action="toggle-um-pass">{p_lbl}</button>
        </div>
      </div>

      <div class="field">
        <label class="field-label">Confirm Password</label>
        <input type="{p_type}" class="input" maxlength="64" data-field="um_pass2" value="{p2}" placeholder="Re-enter password">
      </div>

      <div class="btn-row">
        <button type="button" class="btn btn-primary" data-action="save-user">{submit_lbl}</button>
        {cancel_btn}
      </div>
    </div>
  </section>
</div>"#,
            cnt = self.users.len().max(1),
            rows_html = rows_html,
            form_title = form_title,
            form_desc = form_desc,
            u_val = html_escape(&self.user_form.username),
            user_readonly = user_readonly,
            role_dis = role_dis,
            sel_admin = if self.user_form.role == "admin" || self.user_form.edit_is_primary {
                " selected"
            } else {
                ""
            },
            sel_viewer = if self.user_form.role != "admin" && !self.user_form.edit_is_primary {
                " selected"
            } else {
                ""
            },
            p_type = p_type,
            p1 = html_escape(&self.user_form.password),
            p2 = html_escape(&self.user_form.confirm_password),
            pass_ph = pass_ph,
            p_lbl = p_lbl,
            submit_lbl = submit_lbl,
            cancel_btn = cancel_btn,
        )
    }

    fn render_ota_panel(&self, ro: &str) -> String {
        if self.ota.checking {
            return String::from(
                r#"<div class="ota-result-box">Checking GitHub repository for firmware releases...</div>"#,
            );
        }
        if !self.ota.checked {
            return String::new();
        }
        if !self.ota.ok {
            let err = if self.ota.error.is_empty() {
                String::from("Unable to reach GitHub release metadata.")
            } else {
                html_escape(&self.ota.error)
            };
            return format!(
                r#"<div class="ota-result-box ota-result-err"><strong>Check failed:</strong> {}</div>"#,
                err
            );
        }
        if self.ota.no_release_yet || self.ota.latest_version.is_empty() {
            let msg = if self.ota.notes.is_empty() {
                String::from("Connected to GitHub repository successfully. No published firmware release or manifest.json found yet.")
            } else {
                html_escape(&self.ota.notes)
            };
            return format!(r#"<div class="ota-result-box">{}</div>"#, msg);
        }

        let relay_tag = if self.ota.via_browser_relay {
            r#" <span class="tag tag-blue">Browser-Relayed Mode</span>"#
        } else {
            r#" <span class="tag tag-green">Direct AP HTTPS</span>"#
        };
        let status_badge = if self.ota.has_update {
            r#"<span class="badge badge-blue">New Release Available</span>"#
        } else {
            r#"<span class="badge badge-green">Up to Date</span>"#
        };
        let size_txt = if self.ota.size_bytes > 0 {
            format!(" &bull; {}", format_bytes(self.ota.size_bytes))
        } else {
            String::new()
        };
        let sha_txt = if self.ota.sha256.len() >= 12 {
            format!(
                " &bull; SHA-256: <code>{}</code>",
                html_escape(&self.ota.sha256[..12])
            )
        } else {
            String::new()
        };
        let notes_html = if self.ota.notes.is_empty() {
            String::new()
        } else {
            format!(
                r#"<div class="sub-cell" style="margin-top:4px">{}</div>"#,
                html_escape(&self.ota.notes)
            )
        };
        let action_btn = if !self.ota.download_url.is_empty() {
            let btn_lbl = if self.ota.has_update {
                "Download &amp; Upgrade from GitHub"
            } else {
                "Re-flash Release from GitHub"
            };
            let btn_cls = if self.ota.has_update {
                "btn btn-primary btn-sm"
            } else {
                "btn btn-outline btn-sm"
            };
            format!(
                r#"<div style="margin-top:10px"><button type="button" class="{cls}" data-action="run-github-ota" {ro}>{lbl}</button></div>"#,
                cls = btn_cls,
                ro = ro,
                lbl = btn_lbl
            )
        } else {
            String::new()
        };

        format!(
            r#"<div class="ota-result-box">
  <div style="display:flex;align-items:center;justify-content:space-between;gap:8px;flex-wrap:wrap">
    <div><strong>Latest Release:</strong> <span class="mono">{lat}</span>{relay_tag}</div>
    {status_badge}
  </div>
  <div class="sub-cell" style="margin-top:4px">Asset: <strong class="mono">{fname}</strong>{size_txt}{sha_txt}</div>
  {notes_html}
  {action_btn}
</div>"#,
            lat = html_escape(&self.ota.latest_version),
            relay_tag = relay_tag,
            status_badge = status_badge,
            fname = if self.ota.file_name.is_empty() {
                String::from("Concard_N9_CSB334A-2700.web")
            } else {
                html_escape(&self.ota.file_name)
            },
            size_txt = size_txt,
            sha_txt = sha_txt,
            notes_html = notes_html,
            action_btn = action_btn,
        )
    }

    fn render_system(&self) -> String {
        let ro = if self.is_admin() { "" } else { "disabled" };
        let tz_list = [
            ("UCT", "(GMT+00:00) UTC / London"),
            ("CET-1CEST", "(GMT+01:00) Central Europe"),
            ("UCT-3_1", "(GMT+03:00) Moscow / Riyadh"),
            ("UCT-5", "(GMT+05:00) Tashkent / Karachi"),
            ("UCT-7", "(GMT+07:00) Bangkok, Hanoi, Jakarta"),
            ("CST-8", "(GMT+08:00) Singapore, Taipei, Beijing"),
            ("JST", "(GMT+09:00) Tokyo, Seoul"),
            ("UTC-10", "(GMT+10:00) Sydney, Melbourne"),
            ("EST5EDT", "(GMT-05:00) Eastern Time (US)"),
            ("PST8PDT", "(GMT-08:00) Pacific Time (US)"),
        ];
        let mut tz_opts = String::new();
        let mut matched_tz = false;
        for (val, label) in tz_list {
            let sel = if self.system.time_zone == val {
                matched_tz = true;
                " selected"
            } else {
                ""
            };
            tz_opts.push_str(&format!(
                r#"<option value="{val}"{sel}>{label}</option>"#,
                val = val,
                sel = sel,
                label = label
            ));
        }
        if !matched_tz && !self.system.time_zone.is_empty() {
            let cur = html_escape(&self.system.time_zone);
            tz_opts.insert_str(
                0,
                &format!(r#"<option value="{cur}" selected>Current ({cur})</option>"#),
            );
        }

        let build_info = self.build_badge_str();
        let chk_dis = if !self.is_admin() || self.ota.checking {
            "disabled"
        } else {
            ""
        };
        let chk_lbl = if self.ota.checking {
            "Checking..."
        } else {
            "Check for Updates"
        };
        let ota_panel = self.render_ota_panel(ro);

        format!(
            r#"{vbanner}<div class="grid-2">
  <div class="stack-col">
    <section class="card section-card">
      <div class="card-header">
        <div>
          <h2 class="card-title">Time, Ceiling LED &amp; System Preferences</h2>
          <p class="card-desc">NTP clock synchronization, ceiling ring LED behavior, and user accounts</p>
        </div>
      </div>

      <div class="form-stack">
        <div class="field">
          <label class="field-label">Time Zone</label>
          <select class="input" data-field="s_tz" {ro}>{tz_opts}</select>
        </div>
        <div class="field">
          <label class="field-label">NTP Server</label>
          <input type="text" class="input" maxlength="128" data-field="s_ntp" value="{ntp}" {ro}>
        </div>
        <div class="field">
          <label class="field-label">Ceiling Ring LED Mode (<code>cc_led_mode</code>)</label>
          <select class="input" data-field="s_led_mode" {ro}>
            <option value="0"{led_std}>Standard Mode — Solid Green Online / Solid Red Offline</option>
            <option value="1"{led_night}>Bedroom Night Mode — LED Off Online / Solid Red on Fault</option>
          </select>
        </div>
        <div class="btn-row">
          <button type="button" class="btn btn-primary" data-action="save-system" {ro}>Save System Settings</button>
          <button type="button" class="btn btn-outline" data-action="tab" data-tab="users">Manage Users &rarr;</button>
        </div>
      </div>
    </section>

    <section class="card section-card">
      <div class="card-header">
        <div>
          <h2 class="card-title">System Maintenance</h2>
          <p class="card-desc">Restart Access Point or reset NVRAM configuration to factory defaults</p>
        </div>
      </div>
      <div class="btn-row">
        <button type="button" class="btn btn-outline" data-action="reboot" {ro}>Reboot Access Point</button>
        <button type="button" class="btn btn-danger-outline" data-action="factory-reset" {ro}>Factory Defaults</button>
      </div>
    </section>
  </div>

  <div class="stack-col">
    <section class="card section-card">
      <div class="card-header">
        <div>
          <h2 class="card-title">Online Firmware Update via GitHub</h2>
          <p class="card-desc">Running: <strong>{ver}</strong> &bull; <span class="mono-xs">{build_info}</span></p>
        </div>
        <span class="badge badge-green">GitHub OTA</span>
      </div>

      <div class="form-stack">
        <div class="field">
          <label class="field-label">GitHub Firmware Repository URL</label>
          <input type="text" class="input mono" maxlength="220" data-field="s_ota_repo" value="{ota_repo}" placeholder="https://github.com/fbaduongth/totolink_n9_firmware_custom" {ro}>
        </div>
        <div class="btn-row">
          <button type="button" class="btn btn-primary" data-action="check-github-ota" {chk_dis}>{chk_lbl}</button>
          <button type="button" class="btn btn-outline" data-action="save-ota-repo" {ro}>Save Repository URL</button>
        </div>
        {ota_panel}
      </div>
    </section>

    <section class="card section-card">
      <div class="card-header">
        <div>
          <h2 class="card-title">Manual Firmware Upload</h2>
          <p class="card-desc">Signature-locked to genuine ConCard <code>CSB334A-2700</code> images (anti-rollback active)</p>
        </div>
      </div>
      <div class="form-stack">
        <div class="field">
          <label class="field-label">Select Firmware Image (<code>.web</code> or <code>.trx</code>)</label>
          <input type="file" id="fwFileInput" class="input-file" accept=".web,.trx,.bin" {ro}>
        </div>
        <div>
          <button type="button" class="btn btn-outline" data-action="upload-firmware" {ro}>Upload &amp; Flash Local File</button>
        </div>
      </div>
    </section>
  </div>
</div>"#,
            vbanner = self.render_viewer_banner(),
            ro = ro,
            tz_opts = tz_opts,
            ntp = html_escape(&self.system.ntp_server),
            led_std = if self.system.led_mode != "1" { " selected" } else { "" },
            led_night = if self.system.led_mode == "1" { " selected" } else { "" },
            ver = CURRENT_VERSION_LABEL,
            build_info = build_info,
            ota_repo = html_escape(&self.system.ota_repo_url),
            chk_dis = chk_dis,
            chk_lbl = chk_lbl,
            ota_panel = ota_panel,
        )
    }

    fn render_app(&self) -> String {
        if !self.auth_checked {
            return format!(
                r#"<div class="login-shell">
  <div class="login-center">
    <div style="text-align:center;color:#64748b;font-size:13px">Checking session...</div>
  </div>
  {footer}
</div>"#,
                footer = self.render_footer()
            );
        }

        if !self.authenticated {
            return self.render_login();
        }

        let body = match self.tab {
            Tab::Dashboard => self.render_dashboard(),
            Tab::Wireless => self.render_wireless(),
            Tab::Network => self.render_network(),
            Tab::Users => self.render_users(),
            Tab::System => self.render_system(),
        };
        format!(
            r#"{header}
<main class="container">
  {notice}
  <div id="tabBody">{body}</div>
</main>
{footer}"#,
            header = self.render_header(),
            notice = self.render_notice(),
            body = body,
            footer = self.render_footer()
        )
    }
}

fn encode_form(pairs: &[(&str, String)]) -> String {
    let mut out = String::new();
    for (idx, (k, v)) in pairs.iter().enumerate() {
        if idx > 0 {
            out.push('&');
        }
        out.push_str(&url_encode(k));
        out.push('=');
        out.push_str(&url_encode(v));
    }
    out
}

unsafe fn read_utf8<'a>(ptr: *const u8, len: usize) -> &'a str {
    if ptr.is_null() || len == 0 {
        return "";
    }
    let slice = core::slice::from_raw_parts(ptr, len);
    core::str::from_utf8_unchecked(slice)
}

#[no_mangle]
pub extern "C" fn wasm_alloc(size: usize) -> *mut u8 {
    let mut buf = Vec::<u8>::with_capacity(size);
    let ptr = buf.as_mut_ptr();
    core::mem::forget(buf);
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn wasm_dealloc(ptr: *mut u8, cap: usize) {
    if !ptr.is_null() && cap > 0 {
        drop(Vec::from_raw_parts(ptr, 0, cap));
    }
}

#[no_mangle]
pub extern "C" fn wasm_output_len() -> usize {
    APP.with(|cell| cell.borrow().out_buf.len())
}

#[no_mangle]
pub extern "C" fn wasm_is_authenticated() -> u32 {
    APP.with(|cell| u32::from(cell.borrow().authenticated))
}

#[no_mangle]
pub extern "C" fn wasm_current_tab_is_dashboard() -> u32 {
    APP.with(|cell| {
        let st = cell.borrow();
        u32::from(st.authenticated && st.tab == Tab::Dashboard)
    })
}

#[no_mangle]
pub unsafe extern "C" fn wasm_ingest_auth(ptr: *const u8, len: usize) -> *const u8 {
    let s = read_utf8(ptr, len);
    APP.with(|cell| {
        let mut st = cell.borrow_mut();
        st.ingest_auth_json(s);
        let html = st.render_app();
        st.set_output(html)
    })
}

#[no_mangle]
pub unsafe extern "C" fn wasm_ingest_users(ptr: *const u8, len: usize) -> *const u8 {
    let s = read_utf8(ptr, len);
    APP.with(|cell| {
        let mut st = cell.borrow_mut();
        st.ingest_users_json(s);
        let html = st.render_app();
        st.set_output(html)
    })
}

#[no_mangle]
pub extern "C" fn wasm_set_ota_checking(checking: u32) -> *const u8 {
    APP.with(|cell| {
        let mut st = cell.borrow_mut();
        st.ota.checking = checking != 0;
        let html = st.render_app();
        st.set_output(html)
    })
}

#[no_mangle]
pub extern "C" fn wasm_get_ota_repo_url() -> *const u8 {
    APP.with(|cell| {
        let mut st = cell.borrow_mut();
        let u = if st.system.ota_repo_url.trim().is_empty() {
            String::from(DEFAULT_OTA_REPO)
        } else {
            st.system.ota_repo_url.trim().to_string()
        };
        st.set_output(u)
    })
}

#[no_mangle]
pub extern "C" fn wasm_get_ota_download_info() -> *const u8 {
    APP.with(|cell| {
        let mut st = cell.borrow_mut();
        let info = format!(
            "{}\n{}\n{}",
            st.ota.download_url, st.ota.sha256, st.ota.file_name
        );
        st.set_output(info)
    })
}

#[no_mangle]
pub unsafe extern "C" fn wasm_ingest_ota_json(ptr: *const u8, len: usize) -> *const u8 {
    let s = read_utf8(ptr, len);
    APP.with(|cell| {
        let mut st = cell.borrow_mut();
        st.ingest_ota_json(s);
        let html = st.render_app();
        st.set_output(html)
    })
}

#[no_mangle]
pub extern "C" fn wasm_set_login_busy(busy: u32) -> *const u8 {
    APP.with(|cell| {
        let mut st = cell.borrow_mut();
        st.login_busy = busy != 0;
        let html = st.render_app();
        st.set_output(html)
    })
}

#[no_mangle]
pub unsafe extern "C" fn wasm_select_edit_user(ptr: *const u8, len: usize) -> *const u8 {
    let s = read_utf8(ptr, len);
    APP.with(|cell| {
        let mut st = cell.borrow_mut();
        st.select_edit_user(s);
        let html = st.render_app();
        st.set_output(html)
    })
}

#[no_mangle]
pub extern "C" fn wasm_reset_user_form() -> *const u8 {
    APP.with(|cell| {
        let mut st = cell.borrow_mut();
        st.reset_user_form();
        let html = st.render_app();
        st.set_output(html)
    })
}

#[no_mangle]
pub extern "C" fn wasm_build_login_payload() -> *const u8 {
    APP.with(|cell| {
        let mut st = cell.borrow_mut();
        let res = match st.build_login_payload() {
            Ok(body) => format!("OK\n{}", body),
            Err(e) => format!("ERR\n{}", e),
        };
        st.set_output(res)
    })
}

#[no_mangle]
pub extern "C" fn wasm_build_user_save_payload() -> *const u8 {
    APP.with(|cell| {
        let mut st = cell.borrow_mut();
        let res = match st.build_user_save_payload() {
            Ok(body) => format!("OK\n{}", body),
            Err(e) => format!("ERR\n{}", e),
        };
        st.set_output(res)
    })
}

#[no_mangle]
pub unsafe extern "C" fn wasm_set_tab(ptr: *const u8, len: usize) -> *const u8 {
    let s = read_utf8(ptr, len);
    APP.with(|cell| {
        let mut st = cell.borrow_mut();
        st.tab = Tab::from_str(s);
        let html = st.render_app();
        st.set_output(html)
    })
}

#[no_mangle]
pub unsafe extern "C" fn wasm_set_notice(
    k_ptr: *const u8,
    k_len: usize,
    m_ptr: *const u8,
    m_len: usize,
) -> *const u8 {
    let kind = read_utf8(k_ptr, k_len);
    let msg = read_utf8(m_ptr, m_len);
    APP.with(|cell| {
        let mut st = cell.borrow_mut();
        st.notice_kind = kind.to_string();
        st.notice_msg = msg.to_string();
        let html = st.render_app();
        st.set_output(html)
    })
}

#[no_mangle]
pub unsafe extern "C" fn wasm_update_field(
    k_ptr: *const u8,
    k_len: usize,
    v_ptr: *const u8,
    v_len: usize,
) {
    let k = read_utf8(k_ptr, k_len);
    let v = read_utf8(v_ptr, v_len);
    APP.with(|cell| {
        cell.borrow_mut().update_field(k, v);
    });
}

#[no_mangle]
pub unsafe extern "C" fn wasm_ingest_telemetry(
    state_ptr: *const u8,
    state_len: usize,
    wstat_ptr: *const u8,
    wstat_len: usize,
    ipmon_ptr: *const u8,
    ipmon_len: usize,
    netdev_ptr: *const u8,
    netdev_len: usize,
    sync_forms: u32,
) {
    let state_s = read_utf8(state_ptr, state_len);
    let wstat_s = read_utf8(wstat_ptr, wstat_len);
    let ipmon_s = read_utf8(ipmon_ptr, ipmon_len);
    let netdev_s = read_utf8(netdev_ptr, netdev_len);
    APP.with(|cell| {
        let mut st = cell.borrow_mut();
        if !state_s.is_empty() {
            st.ingest_state_kv(state_s, sync_forms != 0);
        }
        if !ipmon_s.is_empty() {
            st.ingest_ipmonitor(ipmon_s);
        }
        if !wstat_s.is_empty() {
            st.ingest_wstatus(wstat_s);
        }
        if !netdev_s.is_empty() {
            st.ingest_netdev(netdev_s);
        }
    });
}

#[no_mangle]
pub unsafe extern "C" fn wasm_ingest_v2_telemetry(
    ptr: *const u8,
    len: usize,
    sync_forms: u32,
) {
    let s = read_utf8(ptr, len);
    APP.with(|cell| {
        cell.borrow_mut().ingest_v2_telemetry(s, sync_forms != 0);
    });
}

#[no_mangle]
pub unsafe extern "C" fn wasm_ingest_locate(ptr: *const u8, len: usize) {
    let s = read_utf8(ptr, len);
    APP.with(|cell| {
        cell.borrow_mut().ingest_locate_json(s);
    });
}

#[no_mangle]
pub extern "C" fn wasm_tick_locate() -> u32 {
    APP.with(|cell| {
        let mut st = cell.borrow_mut();
        if st.live.locate_remaining_sec > 0 {
            st.live.locate_remaining_sec -= 1;
            1
        } else {
            0
        }
    })
}

#[no_mangle]
pub extern "C" fn wasm_render_locate_btn() -> *const u8 {
    APP.with(|cell| {
        let mut st = cell.borrow_mut();
        let html = st.render_locate_btn();
        st.set_output(html)
    })
}

#[no_mangle]
pub extern "C" fn wasm_render_app() -> *const u8 {
    APP.with(|cell| {
        let mut st = cell.borrow_mut();
        let html = st.render_app();
        st.set_output(html)
    })
}

#[no_mangle]
pub extern "C" fn wasm_render_dashboard_only() -> *const u8 {
    APP.with(|cell| {
        let mut st = cell.borrow_mut();
        let html = st.render_dashboard();
        st.set_output(html)
    })
}

#[no_mangle]
pub extern "C" fn wasm_build_wireless_apply() -> *const u8 {
    APP.with(|cell| {
        let mut st = cell.borrow_mut();
        let res = match st.build_wireless_payload() {
            Ok(body) => format!("OK\n{}", body),
            Err(e) => format!("ERR\n{}", e),
        };
        st.set_output(res)
    })
}

#[no_mangle]
pub extern "C" fn wasm_build_network_apply() -> *const u8 {
    APP.with(|cell| {
        let mut st = cell.borrow_mut();
        let res = match st.build_network_payload() {
            Ok((body, redir)) => format!("OK\n{}\n{}", redir, body),
            Err(e) => format!("ERR\n{}", e),
        };
        st.set_output(res)
    })
}

#[no_mangle]
pub extern "C" fn wasm_build_system_apply() -> *const u8 {
    APP.with(|cell| {
        let mut st = cell.borrow_mut();
        let res = match st.build_system_payload() {
            Ok(body) => format!("OK\n{}", body),
            Err(e) => format!("ERR\n{}", e),
        };
        st.set_output(res)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_v21_pure_english_single_uplink_build_code_and_github_ota() {
        let mut app = AppState::default();
        app.ingest_auth_json(
            r#"{"ok":true,"authenticated":false,"hostname":"LE-TAN","ip":"172.16.6.73","firmver":"3.4.3.9-099_Concard-N9-v2.1"}"#,
        );
        let login_html = app.render_app();
        assert!(login_html.contains("Concard N9 v2.1"));
        assert!(login_html.contains("Build CSB334A-2700-2100 (2026-11-19)"));
        assert!(login_html.contains("Sign In to Access Point"));

        app.ingest_auth_json(
            r#"{"ok":true,"authenticated":true,"user":"admin","role":"admin","hostname":"LE-TAN","ip":"172.16.6.73"}"#,
        );
        app.ingest_v2_telemetry(
            r#"{"ok":true,"version":"Concard N9 v2.1","buildCode":"CSB334A-2700-2100","buildDate":"2026-11-19","firmver_sub":"3.4.3.9-099_Concard-N9-v2.1","computer_name":"LE-TAN","cc_device_loc":"Floor 1 Reception","apMac":"F4:28:53:DC:DB:28","lan_ipaddr_t":"172.16.6.73","lan_gateway_t":"172.16.0.1","gatewayReachable":true,"fallbackAliasActive":false,"uptimeSec":7380,"memTotalKb":126680,"memUsedKb":14200,"memFreeKb":112480,"liveChannel":6,"rt_gmode":5,"cc_cell_breathing":1,"cc_qload_enable":1,"cc_probe_rssi":-82,"cc_fair_queue":1,"cc_l2_shield":1,"cc_led_mode":0,"cc_ota_repo_url":"https://github.com/fbaduongth/totolink_n9_firmware_custom","linkUp":true,"maxSpeedMbps":100,"degraded10M":false,"ports":[{"id":0,"label":"LAN / PoE","up":true,"speed":100,"duplex":"FD"},{"id":1,"label":"LAN1","up":false,"speed":0,"duplex":"NONE"}],"wlanRx":1048576,"wlanTx":2097152,"lanRx":4194304,"lanTx":8388608,"channelBusyPct":17,"falseCca1s":12,"tempC":49,"locateRemainingSec":0,"breathingActive":0,"staCount":1,"stations":[{"mac":"9A:11:22:33:44:55","apIdx":0,"ip":"172.16.6.188","hostname":"","osHint":"Apple iOS","rssi":-48,"rssi0":-48,"rssi1":-51,"txMbps":144,"rxMbps":130,"bw":20,"mode":"802.11n","psm":1,"connSec":615}]}"#,
            true,
        );

        let dash = app.render_app();
        // 1. Header build code instead of duplicate version string
        assert!(dash.contains("Concard N9 v2.1"));
        assert!(dash.contains("Floor 1 Reception &bull; <span class=\"mono-xs\">Build CSB334A-2700-2100 (2026-11-19)</span>"));
        // 2. Single LAN / PoE uplink card (NO LAN1/LAN2/LAN3/LAN4 boxes)
        assert!(dash.contains("LAN / PoE Uplink"));
        assert!(dash.contains("100 Mbps Full-Duplex"));
        assert!(dash.contains("Gateway Reachable (172.16.0.1)"));
        assert!(!dash.contains("LAN1"));
        assert!(!dash.contains("LAN2"));
        assert!(!dash.contains("5-Port Switch"));
        // 3. 100% Pure English UI + OS Fingerprinting
        assert!(dash.contains("Locate LED (30s)"));
        assert!(!dash.contains("Nháy đèn"));
        assert!(!dash.contains("Trống"));
        assert!(dash.contains("Apple iOS"));
        assert!(dash.contains("802.11e QBSS Load"));

        // 4. Wireless tab 100% English + new v2.1 controls
        app.tab = Tab::Wireless;
        let wifi_html = app.render_app();
        assert!(wifi_html.contains("Pure OFDM Mode (Disable 802.11b"));
        assert!(wifi_html.contains("IEEE 802.11e QBSS Load Advertisement"));
        assert!(wifi_html.contains("Probe Rsp RSSI"));
        assert!(!wifi_html.contains("Chế độ sóng"));
        assert!(!wifi_html.contains("Tự động thu hẹp"));

        let w_body = app.build_wireless_payload().unwrap();
        assert!(w_body.contains("cc_qload_enable=1"));
        assert!(w_body.contains("cc_probe_rssi=-82"));

        // 5. System tab GitHub OTA default URL & check result
        app.tab = Tab::System;
        let sys_html = app.render_app();
        assert!(sys_html.contains("https://github.com/fbaduongth/totolink_n9_firmware_custom"));
        assert!(sys_html.contains("Online Firmware Update via GitHub"));

        app.ingest_ota_json(
            r#"{"ok":true,"hasUpdate":true,"viaBrowserRelay":true,"latestVersion":"v2.2","fileName":"Concard_N9_CSB334A-2700_v2.2.web","downloadUrl":"https://github.com/fbaduongth/totolink_n9_firmware_custom/releases/download/v2.2/Concard_N9_CSB334A-2700_v2.2.web","sha256":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef","sizeBytes":6600000,"notes":"New release"}"#,
        );
        let sys_html_upd = app.render_app();
        assert!(sys_html_upd.contains("New Release Available"));
        assert!(sys_html_upd.contains("Download &amp; Upgrade from GitHub"));
    }

    #[test]
    fn manages_users_and_enforces_viewer_readonly() {
        let mut app = AppState::default();
        app.ingest_auth_json(r#"{"ok":true,"authenticated":true,"user":"admin","role":"admin"}"#);
        app.ingest_users_json(
            r#"{"ok":true,"current_user":"admin","current_role":"admin","primary_user":"admin","users":[{"username":"admin","role":"admin","is_primary":true},{"username":"audit1","role":"viewer","is_primary":false}]}"#,
        );
        assert_eq!(app.users.len(), 2);
        app.ingest_auth_json(r#"{"ok":true,"authenticated":true,"user":"audit1","role":"viewer"}"#);
        assert!(app.build_wireless_payload().is_err());
        assert!(app.build_network_payload().is_err());
        assert!(app.build_system_payload().is_err());
    }
}
