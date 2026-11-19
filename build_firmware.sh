#!/usr/bin/env bash
# ==============================================================================
# ConCard Custom Firmware Automated Builder for TOTOLINK N9v1 (CSB334A-2700)
# Target : MediaTek MT7620A (580MHz) / 128MB DDR2 / 8MB SPI NOR Flash
# Release: Concard N9 v2.1 (Build CSB334A-2700-2100)
#   - Zero-CPU 802.11 Assoc IE + BPF DHCP Device Fingerprinting (concard-dhcp-fp)
#   - Full L1/L2/L3 Gateway ICMP/ARP Ceiling LED State Machine + Night Mode
#   - IEEE 802.11e QBSS Load + ProbeRspRssi (-82dBm) Corridor Noise Filter
#   - Per-Host FQ-CoDel (cls_flow) + L2 Broadcast & Wi-Fi Admin Shield
#   - Smart Rescue Alias (192.168.1.165 on br0:fb) + Anti-Rollback Lock
#   - 100% English Single-Port Rust WebAssembly UI + Dual-Path GitHub OTA
# ==============================================================================
set -euo pipefail

export PATH="${HOME:-/root}/.cargo/bin:/root/.cargo/bin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
export CMAKE_POLICY_VERSION_MINIMUM=3.5

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RT_N56U_ROOT="${RT_N56U_ROOT:-/opt/rt-n56u}"
TRUNK_DIR="${RT_N56U_ROOT}/trunk"

echo "[1/4] Checking Padavan source tree & MIPS toolchain at ${RT_N56U_ROOT}..."
if [ ! -d "${TRUNK_DIR}" ]; then
  echo "Cloning hanwckf/rt-n56u into ${RT_N56U_ROOT}..."
  git clone --depth=1 https://github.com/hanwckf/rt-n56u.git "${RT_N56U_ROOT}"
fi

if [ ! -d "${RT_N56U_ROOT}/toolchain-mipsel/toolchain-3.4.x/bin" ]; then
  echo "ERROR: MIPS toolchain not found at ${RT_N56U_ROOT}/toolchain-mipsel/toolchain-3.4.x"
  echo "Build or extract the Padavan mipsel-linux-uclibc toolchain first (dl_toolchain.sh)."
  exit 1
fi

echo "[2/4] Applying ConCard N9 v2.1 (CSB334A-2700-2100) BSP, Kernel & WASM UI..."
TRUNK_ENV="${TRUNK_DIR}" python3 "${SCRIPT_DIR}/apply_concard_n9v1_bsp.py"

echo "[3/4] Compiling Kernel 3.4.113, mt76x2_ap (802.11k/r/v + QBSS Load + Vendor IE), and Userspace..."
cd "${TRUNK_DIR}"
./build_firmware_modify CSB334A-2700

echo "[4/4] Verifying romfs binaries, modules.dep & packaging v2.1 release artifacts..."
test -s "${TRUNK_DIR}/romfs/lib/modules/3.4.113/modules.dep"
grep -q "mt76x2_ap.ko:" "${TRUNK_DIR}/romfs/lib/modules/3.4.113/modules.dep"
test -x "${TRUNK_DIR}/romfs/usr/sbin/httpd"
test -e "${TRUNK_DIR}/romfs/sbin/detect_link"
test -x "${TRUNK_DIR}/romfs/usr/sbin/concard-locate-led"
test -x "${TRUNK_DIR}/romfs/usr/sbin/concard-watchdog"
test -x "${TRUNK_DIR}/romfs/usr/sbin/concard-dhcp-fp"
test -x "${TRUNK_DIR}/romfs/usr/sbin/concard-ota"
test -s "${TRUNK_DIR}/romfs/www/ap_ui.wasm"

TRX_FILE="$(ls -1t "${TRUNK_DIR}"/images/CSB334A-2700*.trx | head -n 1)"
FW_SIZE="$(wc -c < "${TRX_FILE}" | tr -d ' ')"
if [ "${FW_SIZE}" -gt 7012352 ]; then
  echo "ERROR: Firmware image (${FW_SIZE} bytes) exceeds 6,848 KB SPI NOR partition limit!"
  exit 1
fi

mkdir -p "${SCRIPT_DIR}/releases"
cp -f "${TRX_FILE}" "${SCRIPT_DIR}/releases/Concard_N9_CSB334A-2700_v2.1.trx"
cp -f "${TRX_FILE}" "${SCRIPT_DIR}/releases/Concard_N9_CSB334A-2700_v2.1.web"

FW_SHA256="$(sha256sum "${SCRIPT_DIR}/releases/Concard_N9_CSB334A-2700_v2.1.web" | awk '{print $1}')"

cat > "${SCRIPT_DIR}/releases/manifest.json" <<EOF
{
  "product": "Concard N9",
  "boardId": "CSB334A-2700",
  "version": "2.1",
  "displayVersion": "Concard N9 v2.1",
  "buildCode": "CSB334A-2700-2100",
  "buildDate": "2026-11-19",
  "fileName": "Concard_N9_CSB334A-2700_v2.1.web",
  "sizeBytes": ${FW_SIZE},
  "sha256": "${FW_SHA256}",
  "downloadUrl": "https://github.com/fbaduongth/totolink_n9_firmware_custom/releases/download/v2.1/Concard_N9_CSB334A-2700_v2.1.web",
  "notes": "Concard N9 v2.1: Zero-CPU OS Fingerprinting, L3 Gateway Ceiling LED + Night Mode, 802.11e QBSS Load, ProbeRspRssi -82dBm, Per-Host FQ-CoDel, L2 & Wi-Fi Admin Shield, 100% English Single-Port UI, GitHub OTA."
}
EOF
cp -f "${SCRIPT_DIR}/releases/manifest.json" "${SCRIPT_DIR}/manifest.json"

(
  cd "${SCRIPT_DIR}/releases"
  sha256sum Concard_N9_CSB334A-2700_v2.1.trx Concard_N9_CSB334A-2700_v2.1.web > SHA256SUMS
  if [ -f Concard_N9_CSB334A-2700_v2.0.web ]; then
    sha256sum Concard_N9_CSB334A-2700_v2.0.trx Concard_N9_CSB334A-2700_v2.0.web >> SHA256SUMS 2>/dev/null || true
  fi
  cp -f SHA256SUMS SHA256SUMS.txt
)

ls -lh "${SCRIPT_DIR}/releases"/Concard_N9_CSB334A-2700_v2.1.* "${SCRIPT_DIR}/releases/manifest.json"
cat "${SCRIPT_DIR}/releases/SHA256SUMS"
echo "=== CONCARD N9 v2.1 BUILD COMPLETE ==="
