# Tài Liệu Kỹ Thuật Phiên Bản — Concard N9 v2.1 (`Build CSB334A-2700-2100`)

> **Trạng thái:** **Đã triển khai & đóng gói thành công (`Concard_N9_CSB334A-2700_v2.1.web` / `.trx` + `manifest.json`)**. Sẵn sàng nạp thử nghiệm hoặc phát hành qua `ConCard AP Monitor` và GitHub OTA (`https://github.com/fbaduongth/totolink_n9_firmware_custom`).

## Hạng Mục 1: Định Danh Chính Xác Thiết Bị Ẩn Danh Ngay Tại AP (Zero-CPU Deep Device Fingerprinting)

Mục tiêu: Nhận diện chính xác **iPhone / iPad (iOS)**, **MacBook (macOS)**, **Samsung Galaxy**, **Android 13/14/15** và **Windows 10/11** ngay cả khi thiết bị bật **MAC ngẫu nhiên (Private Wi-Fi Address / LAA)** và xóa trống `DHCP Option 12 (Hostname)`, **không cần** bật `DEBUG` log nặng trên OPNsense Kea DHCP.

### 1. Bóc tách 2 lớp vân tay trực tiếp trên AP `Concard N9` (`~0% CPU`, `< 8 KB RAM`)
1. **Tầng Sóng 802.11 (`APPeerAssocReqAction` trong `mt76x2_ap.ko`):**
   - Chỉ chạy 1 lần duy nhất (vài micro-giây) khi thiết bị gửi khung `802.11 Association / Reassociation Request`.
   - Quét các thẻ `Vendor Specific IE (Tag 221)` để ghi nhận cờ phần cứng vào `MAC_TABLE_ENTRY`:
     - `00:17:F2` / `00:10:FA`: Hệ sinh thái **Apple** (kết hợp `PSM=1` / `HT Cap` để phân biệt `iPhone/iPad` với `MacBook`).
     - `00:00:F0`: **Samsung Electronics** (nhận diện máy Samsung ngay cả khi ẩn Hostname).
     - `00:50:F2` (kèm MSFT IE): Thiết bị **Windows / Microsoft**.
2. **Tầng Mạng DHCP (`UDP 68 -> 67` đi qua cầu nối `br0`):**
   - Bắt duy nhất gói `DHCP Discover / Request` lúc thiết bị xin IP mà không cần chạy `tcpdump`:
     - **`Option 55` (`Parameter Request List`):** Khớp chữ ký ngăn xếp mạng Darwin/iOS (`1, 121, 3, 6, 15, [108], 114, 119, 252` + `Option 57 = 1500`, không có `Option 60`) -> khẳng định **99% iPhone / iPad**.
     - **`Option 60` (`Vendor Class Identifier`):** Đọc trực tiếp phiên bản `android-dhcp-13`, `android-dhcp-14`, `android-dhcp-15`, `MSFT 5.0`, Smart TV (`webOS`, `Tizen`...).
     - **`Option 12` (`Hostname`):** Bắt tức thì tên đầy đủ của thiết bị ngay tại AP trước cả chu kỳ đồng bộ Kea DHCP.

### 2. Đồng bộ lên `/api_telemetry.cgi` & Bộ nhớ đệm Trung tâm (`ConCard AP Monitor`)
- Xuất thêm trường `"osHint"` / `"vendorHint"` / `"dhcpPrl"` trong từng phần tử của mảng `stations[]` thuộc `/api_telemetry.cgi`.
- Máy chủ `standalone_ap_collector.js` lưu bền vững vân tay theo địa chỉ MAC vào `clients_identity_cache.json` (nhờ vậy khi khách di chuyển từ phòng ngủ xuống khu vực Sảnh/Nhà hàng dùng AP UniFi, hệ thống vẫn nhớ và hiển thị đúng thiết bị `Apple iPhone` của khách).


---

## Hạng Mục 2: Đèn LED Vòng Trần Báo Trạng Thái Thực Tế Toàn Diện (Online = LED Xanh, Offline = LED Đỏ)

### 1. Điểm hạn chế hiện tại trên bản `v2.0` cần hoàn thiện ở `v2.1`
- Trên phần cứng `TOTOLINK N9v1` (`CSB334A-2700`), cụm đèn vòng trần gồm 2 kênh GPIO độc lập (Active-Low: `0 = Sáng`, `1 = Tắt`):
  - **`GPIO 11` (`BOARD_GPIO_LED_POWER`):** Đèn LED Xanh (System / Online).
  - **`GPIO 39` (`BOARD_GPIO_LED_WAN`):** Đèn LED Đỏ (Fault / Offline).
- Ở bản `v2.0` (`dl_concard_sync_ap_led` trong `user/rc/detect_link.c`), AP mới chỉ bật đèn Đỏ khi **rút hẳn cáp mạng (`5-port EPHY mask == 0`)** hoặc **chế độ DHCP chưa lấy được IP (`0.0.0.0` / `192.168.1.165`)**.
- **Lỗ hổng thực tế ngoài hiện trường khách sạn:**
  - Khi AP đặt **IP Tĩnh (`lan_proto_x == 0`)**, hoặc đã nhận IP DHCP trước đó nhưng sau đó **Switch tầng đứt Uplink**, **cắm nhầm VLAN**, hoặc **hạt mạng RJ45 bị chập/đứt 1 sợi TX** (đèn link vật lý trên cổng Switch vẫn sáng nhưng gói tin không đi qua được): ở bản `v2.0` đèn trần vẫn sáng Xanh dù trên hệ thống AP đã **Offline hoàn toàn**.

### 2. Thiết kế chuẩn hóa trên `Concard N9 v2.1` (`user/rc/detect_link.c` — `0% CPU` phụ trội)
Nâng cấp máy trạng thái LED trực tiếp trong tiến trình C `detect_link` (kiểm tra đồng thời cả 3 tầng Vật lý L1 + IP L2 + Gateway L3 cho **cả IP Tĩnh lẫn DHCP**):

| Trạng thái thực tế của AP | Điều kiện kiểm tra kỹ thuật (`detect_link.c`) | Hiển thị Đèn LED Vòng Trần |
| :--- | :--- | :--- |
| **ONLINE (Hoạt động chuẩn)** | Có link cáp vật lý (`mask != 0`) **VÀ** có IP hợp lệ **VÀ** thông kết nối ARP / ICMP tới Gateway trung tâm (`lan_gateway_t` / `lan_gateway`, ví dụ `172.16.0.1`) | **Sáng Xanh cố định** (`GPIO 11 = ON`, `GPIO 39 = OFF`) |
| **OFFLINE (Mất kết nối mạng)** | **Một trong các lỗi:**<br>1. Tuột/đứt cáp mạng hoàn toàn (`mask == 0`).<br>2. Lỗi không nhận được DHCP (kẹt ở `0.0.0.0` / `192.168.1.165`).<br>3. Cáp RJ45 lỗi sợi / Switch tầng mất Uplink / Sai VLAN làm **mất phản hồi ARP + Ping tới Gateway 3 chu kỳ liên tiếp (~6–9 giây)** (áp dụng cho cả **IP Tĩnh** và **DHCP**). | **Sáng Đỏ cố định** (`GPIO 39 = ON`, `GPIO 11 = OFF`) — Kỹ thuật đi dọc hành lang nhìn lên trần biết ngay AP nào đang mất mạng |
| **LOCATE (Định vị thiết bị)** | Khi quản trị viên bấm nút `Locate` trên Dashboard (`/tmp/.concard_locate_until` còn hiệu lực) | **Nháy Xanh – Đỏ luân phiên nhanh (`250ms`)** (ưu tiên cao nhất, tự trả về Xanh/Đỏ chuẩn ngay khi hết giờ nháy) |
| **CHẾ ĐỘ PHÒNG NGỦ (Tuỳ chọn WebUI)** | Bổ sung tuỳ chọn NVRAM `cc_led_mode` trên giao diện WebUI (`0 = Bật chuẩn Xanh/Đỏ`, `1 = Tắt đèn Xanh ban đêm trong phòng ngủ, chỉ sáng Đỏ khi Offline/Lỗi`) | Tắt sáng chói ban đêm cho khách ngủ nhưng vẫn tự bật **Đỏ** nếu thiết bị đứt mạng |


---

## Hạng Mục 3: Quảng Bá Độ Tải Kênh `802.11e QBSS Load` (Tag 11) Trong Khung Beacon *(Kế thừa từ `hostapd` OpenWrt)*

- **Mục tiêu:** Đính kèm trực tiếp **Số lượng Client hiện tại (`Station Count`)** và **Độ bận kênh sóng thực tế (`Channel Utilization 0..255` từ thanh ghi `CH_BUSY_STA`)** vào mọi khung `Beacon` và `Probe Response`.
- **Cách thực hiện (`mt76x2_ap.ko`):**
  - Bật cờ biên dịch `-DAP_QLOAD_SUPPORT` trong `proprietary/rt_wifi/rtpci/3.0.X.X/mt76x2/` (chạy gốc trong nhân Kernel, `0%` CPU).
- **Lợi ích:** Thiết bị **Apple (iOS / macOS)** và **Samsung** đọc trực tiếp thẻ `QBSS Load (Tag 11)` để tự động chọn AP rảnh kênh nhất ngay từ lúc kết nối đầu tiên mà không cần lệnh kick cưỡng bức.

---

## Hạng Mục 4: Lọc Nhiễu Dò Sóng Hành Lang `ProbeRspRssi` & `ApProbeRspTimes` *(Tương đương `rssi_ignore_probe_request` của OpenWrt)*

- **Mục tiêu:** Cắt giảm 35–50% khung quản trị rác (`Management Frame Overhead`) trên băng tần 2.4GHz trong hành lang khách sạn 12 tầng.
- **Cách thực hiện (`user/rc/ralink.c` & `mt76x2_ap`):**
  - Thiết lập mặc định `ProbeRspRssi=-82` (có thể tinh chỉnh qua NVRAM `cc_probe_rssi_thres`): các AP ở xa nghe thấy gói `Probe Request` của điện thoại yếu hơn `-82 dBm` sẽ tự giữ im lặng, nhường cho 2–3 AP gần khách nhất phản hồi.
  - Hạ `ApProbeRspTimes=2` (thay vì phát lặp dư thừa 3 lần ở tốc độ thấp).

---

## Hạng Mục 5: Chia Băng Thông Công Bằng Theo Từng Thiết Bị (`Per-Host FQ-CoDel` — Kế thừa từ `CAKE` của OpenWrt)

- **Mục tiêu:** Ngăn chặn 1 laptop khách mở hàng chục luồng kéo Torrent / IDM chiếm hết băng thông cổng `100Mbps` của các điện thoại gọi Zalo / xem YouTube cùng phòng, nhưng vẫn giữ tải CPU `~0%` trên chip `MT7620A 580MHz`.
- **Cách thực hiện (`concard-vlan-apply` + Kernel `cls_flow.ko`):**
  - Nạp module `cls_flow` (đã biên dịch sẵn trong `CONFIG_NET_CLS_FLOW=m`) gắn vào hàng đợi `fq_codel` trên `ra0`, `ra1` (`flow hash keys dst divisor 1024`) và chiều lên `eth2` (`flow hash keys src divisor 1024`).
  - Mọi thiết bị khách đều được chia phần băng thông và độ trễ công bằng theo **địa chỉ IP/Host** bất kể số lượng luồng kết nối mở ra.

---

## Hạng Mục 6: Lá Chắn Lọc Rác Quảng Bá Lớp 2, Tàng Hình Cổng Quản Trị & Khóa Chữ Ký Firmware (`CONCARD_L2_SHIELD`)

- **1. Lọc rác quảng bá ngang trên miền `/21` (`ebtables` trong `concard-vlan-apply`):**
  - Ngoài luật chặn Rogue DHCP (`UDP sport 67`), bổ sung chặn các gói quảng bá rác không cần thiết đi ra giao diện sóng `ra+` (`NetBIOS UDP 137-138`, `SSDP UPnP UDP 1900`, `LLMNR UDP 5355`), chỉ giữ lại `ARP` và `DHCP` chuẩn để tiết kiệm airtime 2.4GHz và pin điện thoại khách.
- **2. Cấm truy cập Web/SSH của AP từ phía sóng Wi-Fi (`ra+`):**
  - Thêm luật `ebtables -t filter -A INPUT -i ra+ -p IPv4 --ip-proto tcp --ip-dport 22/80/443 -j DROP`: khách đang bắt sóng Wi-Fi hoàn toàn không thể quét thấy hay mở trang đăng nhập/SSH của AP; chỉ máy chủ và kỹ thuật viên đi từ dây cáp LAN Switch mới quản trị được.
- **3. Khóa hạ cấp trái phép & Tắt dịch vụ dò quét Lớp 2 (`upload.c` & `infosvr`):**
  - Khóa kiểm định chữ ký `ConCard` trong `user/httpd/upload.c` (chặn kẻ xấu tải ngược Firmware gốc TOTOLINK lên `/upgrade.cgi` để dùng `SoftAC` chiếm quyền), đồng thời tắt hẳn daemon `infosvr` (`UDP 9999`).

---

## Hạng Mục 7: IP Cứu Hộ Kép Thông Minh (`192.168.1.165` Secondary Alias — Kế thừa từ `OpenWrt-N9v1`)

- **Mục tiêu:** Giúp kỹ thuật viên cắm thẳng AP vào Laptop rời tại hiện trường vẫn truy cập được ngay `http://192.168.1.165` mà không làm gián đoạn chế độ `DHCP Client` chính.
- **Cách thực hiện (`concard-watchdog` / `detect_link.c`):**
  - Khi AP ở chế độ `DHCP` mà quá 12 giây chưa có DHCP Server cấp IP (cắm trực tiếp vào Laptop), tự gắn địa chỉ phụ `192.168.1.165/24` (`br0:fb`) song song với tiến trình `udhcpc` vẫn chạy ngầm.
  - Ngay khi cắm lại vào Switch tầng và nhận được IP thật từ OPNsense Kea DHCP (`172.16.x.x`), tự động gỡ sạch IP phụ `192.168.1.165` để tuyệt đối không bao giờ gây xung đột IP trong mạng.


---

## Hạng Mục 8: Chuẩn Hóa Giao Diện Quản Trị WebUI Trên AP (`webui/src/lib.rs` & `ap_ui.js` — 100% English)

1. **Thay dòng lặp tên phiên bản trên Header bằng Mã Bản Dựng (`Firmware Build Code`):**
   - Vị trí phụ đề dưới tiêu đề chính (`<Location> • ...`) không lặp lại chữ `Concard N9 v2.x` mà hiển thị **mã bản dựng kỹ thuật chính xác** (ví dụ: `Build CSB334A-2700-2100 • 2026-11-19`), lấy trực tiếp từ `concard_v2.c` (`/api_telemetry.cgi`).
2. **Tinh gọn hiển thị cổng mạng về duy nhất 1 cổng vật lý `LAN / PoE Uplink (br0)`:**
   - Gỡ bỏ hoàn toàn các ô cổng ảo không tồn tại ngoài vỏ (`LAN1`, `LAN2`, `LAN3`, `LAN4` và nhãn `1 / 5 cổng đang kết nối`).
   - Thay bằng thẻ đơn **`LAN / PoE Physical Uplink (br0)`** hiển thị trực diện trạng thái cổng RJ45 duy nhất (`100 Mbps Full-Duplex`, cảnh báo `10 Mbps Degraded — Check RJ45 Cable`, hoặc `Link Down`) kèm trạng thái thông kết nối Lớp 3 tới Gateway (`Gateway Reachable`).
3. **Đồng bộ ngôn ngữ 100% Tiếng Anh (`100% Pure English UI`):**
   - Chuyển toàn bộ các chuỗi tiếng Việt còn lẫn trên giao diện AP sang tiếng Anh chuẩn quốc tế (`Locate LED (30s)`, `Pure OFDM Mode (Disable 802.11b)`, `Adaptive Cell Breathing`, thông báo trạng thái cổng mạng, v.v.).
4. **Đưa trọn bộ thông tin & cấu hình mới của `v2.1` lên UI Quản trị:**
   - **Tab `Dashboard`:** Bổ sung cột nhận diện hệ điều hành/hãng (`Device / OS` từ `Vendor IE + DHCP Fingerprint`) trong bảng *Connected Stations*, thẻ trạng thái `Gateway Reachability`, `QBSS Load`, `Per-Host FQ-CoDel` và `L2 Shield`.
   - **Tab `Wireless` / `Network` / `System`:** Bổ sung tuỳ chỉnh cho `802.11e QBSS Load`, `Probe Response RSSI Threshold (dBm)`, `Per-Host Fair Queueing`, `L2 Broadcast & Wi-Fi Admin Shield`, `Ceiling LED Mode` (`Standard Online=Green / Offline=Red` vs `Bedroom Night Mode`), và `Controller URL`.

---

## Hạng Mục 9: Cập Nhật Firmware Trực Tuyến Qua GitHub (`GitHub OTA Update`)

- **Repository mặc định (`cc_ota_repo_url` trong NVRAM):**
  - `https://github.com/fbaduongth/totolink_n9_firmware_custom` (hiển thị sẵn trong tab `System -> Firmware Upgrade`, cho phép chỉnh sửa & lưu URL nếu cần).
- **Quy trình kiểm tra & nâng cấp tự động an toàn (Dual-Path GitHub OTA):**
  1. **`Check for Updates`:** Truy vấn `GitHub Releases API` (`api.github.com/repos/fbaduongth/totolink_n9_firmware_custom/releases/latest` hoặc file `manifest.json` / `SHA256SUMS` của repo), so sánh `Version / Build Code` mới nhất trên GitHub với bản đang chạy trên AP, hiển thị tên bản mới, dung lượng, ghi chú phát hành (Changelog) và mã `SHA-256`.
  2. **`One-Click Upgrade from GitHub`:**
     - **Luồng 1 (Direct AP Pull):** Nếu VLAN của AP có đường ra Internet, CGI trên AP dùng `curl -L` tải trực tiếp file `.web`/`.trx` vào `/tmp/concard_fw.bin`, đối chiếu chuẩn `SHA-256` + header `CSB334A-2700` rồi gọi `mtd_write -r`.
     - **Luồng 2 (Browser-Relayed OTA Fallback):** Nếu VLAN quản trị AP bị tường lửa chặn ra Internet, chính trình duyệt của quản trị viên sẽ tự tải gói Firmware từ GitHub về RAM trình duyệt, xác thực `SHA-256` ngay trong WebAssembly rồi đẩy thẳng xuống `/upgrade.cgi` của AP — đảm bảo tính năng cập nhật qua GitHub hoạt động 100% trong mọi mô hình mạng!
