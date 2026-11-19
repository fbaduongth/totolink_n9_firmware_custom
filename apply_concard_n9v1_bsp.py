#!/usr/bin/env python3
"""
ConCard Enterprise Custom Firmware BSP & 802.11k/r/v + v2.1 Installer
Target Device : TOTOLINK N9v1 (CaryStudio CSB334A, Rev 0154 — ProductID CSB334A-2700)
SoC / Memory  : MediaTek MT7620A (580MHz) / 128MB DDR2 / 8MB SPI NOR Flash
Radio Design  : 2.4GHz 2T2R 300Mbps + External PA & External LNA (Factory @ 0x40000)
Release       : Concard N9 v2.1 (Build CSB334A-2700-2100)
"""

import os
import shutil
import subprocess
import sys

TRUNK = os.environ.get("TRUNK_ENV", "/opt/rt-n56u/trunk")
WIN_CUSTOM_DIR = os.path.dirname(os.path.abspath(__file__))
WIN_BSP_DIR = os.path.join(WIN_CUSTOM_DIR, "bsp")
BOARD_ID = "CSB334A-2700"
BOARD_DIR = os.path.join(TRUNK, "configs/boards", BOARD_ID)
TEMPLATE_PATH = os.path.join(TRUNK, "configs/templates", f"{BOARD_ID}.config")


def set_kconfig_line(text: str, symbol: str, val) -> str:
    """Set or disable a kernel config symbol cleanly."""
    lines = text.splitlines()
    out = []
    matched = False
    prefix_eq = f"{symbol}="
    prefix_unset = f"# {symbol} is not set"
    if val is None or val is False or val == "n":
        new_line = f"# {symbol} is not set"
    else:
        new_line = f"{symbol}={val}"

    for line in lines:
        if line.startswith(prefix_eq) or line == prefix_unset:
            if not matched:
                out.append(new_line)
                matched = True
        else:
            out.append(line)
    if not matched:
        out.append(new_line)
    return "\n".join(out) + "\n"


def build_kernel_config() -> str:
    oye_cfg_path = os.path.join(TRUNK, "configs/boards/OYE-001/kernel-3.4.x.config")
    with open(oye_cfg_path, "r", encoding="utf-8") as f:
        cfg = f.read()

    # 1. Exact 8MB SPI Flash layout matching stock TOTOLINK N9v1:
    #    0x000000..0x030000 Bootloader (192K)
    #    0x030000..0x040000 Config     (64K)
    #    0x040000..0x050000 Factory    (64K - RF EEPROM)
    #    0x050000..0x700000 Firmware   (6848K - Kernel + SquashFS RootFS)
    #    0x700000..0x800000 Storage    (1024K - /etc/storage, matches stock UserData)
    cfg = set_kconfig_line(cfg, "CONFIG_RT2880_FLASH_16M", None)
    cfg = set_kconfig_line(cfg, "CONFIG_RT2880_FLASH_8M", "y")
    cfg = set_kconfig_line(cfg, "CONFIG_MTD_STORE_PART_SIZ", "0x100000")

    # 2. Pinmux & External PA + External LNA RF Front-End (CRITICAL for N9v1 TX/RX power!)
    cfg = set_kconfig_line(cfg, "CONFIG_RALINK_GPIOMODE_PA_G", None)
    cfg = set_kconfig_line(cfg, "CONFIG_RALINK_GPIOMODE_EPHY", "y")
    cfg = set_kconfig_line(cfg, "CONFIG_RALINK_GPIOMODE_UARTF", "y")

    cfg = set_kconfig_line(cfg, "CONFIG_INTERNAL_PA_INTERNAL_LNA", None)
    cfg = set_kconfig_line(cfg, "CONFIG_INTERNAL_PA_EXTERNAL_LNA", None)
    cfg = set_kconfig_line(cfg, "CONFIG_EXTERNAL_PA_EXTERNAL_LNA", "y")
    cfg = set_kconfig_line(cfg, "CONFIG_RT2860V2_AP_RTMP_INTERNAL_TX_ALC", None)

    cfg = set_kconfig_line(cfg, "CONFIG_MT76X2_AP_INTERNAL_PA_INTERNAL_LNA", None)
    cfg = set_kconfig_line(cfg, "CONFIG_MT76X2_AP_INTERNAL_PA_EXTERNAL_LNA", None)
    cfg = set_kconfig_line(cfg, "CONFIG_MT76X2_AP_EXTERNAL_PA_EXTERNAL_LNA", "y")
    cfg = set_kconfig_line(cfg, "CONFIG_MT76X2_AP_RTMP_INTERNAL_TX_ALC", None)

    # 3. Enterprise High-Density Wi-Fi & Fast Roaming capabilities (+ strip unused ceiling-AP bloat)
    cfg = set_kconfig_line(cfg, "CONFIG_EXT4_FS", None)
    cfg = set_kconfig_line(cfg, "CONFIG_SOUND", None)
    cfg = set_kconfig_line(cfg, "CONFIG_MEDIA_SUPPORT", None)
    cfg = set_kconfig_line(cfg, "CONFIG_MMC", None)
    cfg = set_kconfig_line(cfg, "CONFIG_RT_MAX_CLIENTS", "64")
    cfg = set_kconfig_line(cfg, "CONFIG_RT_DOT11R_FT", "y")
    cfg = set_kconfig_line(cfg, "CONFIG_RT_DOT11K_RRM", "y")
    cfg = set_kconfig_line(cfg, "CONFIG_RT_ED_MONITOR", "y")
    cfg = set_kconfig_line(cfg, "CONFIG_RT_IGMP_SNOOP", "y")
    cfg = set_kconfig_line(cfg, "CONFIG_MT76X2_AP_MAC_REPEATER", None)
    cfg = set_kconfig_line(cfg, "CONFIG_MT76X2_AP_MWDS", None)

    # 4. Built-in FQ-CoDel & cls_flow Per-Host Packet Scheduler (zero-latency anti-bufferbloat)
    net_sched_block = """CONFIG_NET_SCHED=y

#
# Queueing/Scheduling
#
# CONFIG_NET_SCH_CBQ is not set
CONFIG_NET_SCH_HTB=m
CONFIG_NET_SCH_HFSC=m
CONFIG_NET_SCH_PRIO=m
# CONFIG_NET_SCH_MULTIQ is not set
CONFIG_NET_SCH_RED=m
# CONFIG_NET_SCH_SFB is not set
CONFIG_NET_SCH_SFQ=m
# CONFIG_NET_SCH_TEQL is not set
# CONFIG_NET_SCH_TBF is not set
CONFIG_NET_SCH_GRED=m
CONFIG_NET_SCH_DSMARK=m
# CONFIG_NET_SCH_NETEM is not set
# CONFIG_NET_SCH_DRR is not set
# CONFIG_NET_SCH_MQPRIO is not set
# CONFIG_NET_SCH_CHOKE is not set
# CONFIG_NET_SCH_QFQ is not set
# CONFIG_NET_SCH_INGRESS is not set
# CONFIG_NET_SCH_PLUG is not set
CONFIG_NET_SCH_CODEL=y
CONFIG_NET_SCH_FQ_CODEL=y

#
# Classification
#
CONFIG_NET_CLS=y
CONFIG_NET_CLS_BASIC=m
CONFIG_NET_CLS_TCINDEX=m
# CONFIG_NET_CLS_ROUTE4 is not set
CONFIG_NET_CLS_FW=m
CONFIG_NET_CLS_U32=m
# CONFIG_CLS_U32_PERF is not set
CONFIG_CLS_U32_MARK=y
# CONFIG_NET_CLS_RSVP is not set
# CONFIG_NET_CLS_RSVP6 is not set
CONFIG_NET_CLS_FLOW=m
CONFIG_NET_EMATCH=y
CONFIG_NET_EMATCH_STACK=32
CONFIG_NET_EMATCH_CMP=m
CONFIG_NET_EMATCH_NBYTE=m
CONFIG_NET_EMATCH_U32=m
CONFIG_NET_EMATCH_META=m
# CONFIG_NET_EMATCH_TEXT is not set
# CONFIG_NET_CLS_ACT is not set
# CONFIG_NET_CLS_IND is not set
CONFIG_NET_SCH_FIFO=y"""

    if "# CONFIG_NET_SCHED is not set" in cfg:
        cfg = cfg.replace("# CONFIG_NET_SCHED is not set", net_sched_block, 1)

    return cfg


def build_template_config() -> str:
    return """### Target Vendor/Product (support only Ralink RT3883/MT7620/MT7621/MT7628)
CONFIG_VENDOR=Ralink
CONFIG_PRODUCT=MT7620

### Target ProductID (board select, max 12 symbols - matches TOTOLINK CSB334A-<svn>)
CONFIG_FIRMWARE_PRODUCT_ID="CSB334A-2700"

### Linux kernel and toolchain
CONFIG_LINUXDIR=linux-3.4.x

############################################################
### Linux kernel configuration
############################################################
#CONFIG_FIRMWARE_TYPE_ROOTFS_IN_RAM=y
#CONFIG_FIRMWARE_CPU_600MHZ=y
#CONFIG_FIRMWARE_CPU_SLEEP=y

### Kernel driver select for WiFi AP 2.4GHz (3.0 = MT7620 v3.0.4.0 with 802.11k/r)
CONFIG_FIRMWARE_WIFI2_DRIVER=3.0

CONFIG_FIRMWARE_ENABLE_IPV6=y
CONFIG_FIRMWARE_ENABLE_USB=y
CONFIG_FIRMWARE_ENABLE_UFSD=n
CONFIG_FIRMWARE_INCLUDE_ANTFS=n
CONFIG_FIRMWARE_ENABLE_FAT=y
CONFIG_FIRMWARE_ENABLE_EXFAT=n
CONFIG_FIRMWARE_ENABLE_EXT2=n
CONFIG_FIRMWARE_ENABLE_EXT3=n
CONFIG_FIRMWARE_ENABLE_EXT4=n
CONFIG_FIRMWARE_ENABLE_XFS=n
CONFIG_FIRMWARE_ENABLE_FUSE=n
CONFIG_FIRMWARE_ENABLE_SWAP=n
CONFIG_FIRMWARE_INCLUDE_UVC=n
CONFIG_FIRMWARE_INCLUDE_HID=n
CONFIG_FIRMWARE_INCLUDE_SERIAL=n
CONFIG_FIRMWARE_INCLUDE_AUDIO=n
CONFIG_FIRMWARE_INCLUDE_QOS=y
CONFIG_FIRMWARE_INCLUDE_IMQ=n
CONFIG_FIRMWARE_INCLUDE_IFB=n
CONFIG_FIRMWARE_INCLUDE_IPSET=n
CONFIG_FIRMWARE_INCLUDE_NFSD=n
CONFIG_FIRMWARE_INCLUDE_NFSC=n
CONFIG_FIRMWARE_INCLUDE_CIFS=n

############################################################
### Userspace configuration (Lean Enterprise AP + Diagnostics)
############################################################
CONFIG_FIRMWARE_INCLUDE_LANG_CN=n
CONFIG_FIRMWARE_INCLUDE_NTFS_3G=n
CONFIG_FIRMWARE_INCLUDE_LPRD=n
CONFIG_FIRMWARE_INCLUDE_U2EC=n
CONFIG_FIRMWARE_INCLUDE_TCPDUMP=n
CONFIG_FIRMWARE_INCLUDE_HDPARM=n
CONFIG_FIRMWARE_INCLUDE_PARTED=n
CONFIG_FIRMWARE_INCLUDE_SMBD=n
CONFIG_FIRMWARE_INCLUDE_SMBD36=n
CONFIG_FIRMWARE_INCLUDE_WINS=n
CONFIG_FIRMWARE_INCLUDE_SMBD_SYSLOG=n
CONFIG_FIRMWARE_INCLUDE_FTPD=n
CONFIG_FIRMWARE_INCLUDE_RPL2TP=n
CONFIG_FIRMWARE_INCLUDE_EAP_PEAP=n
CONFIG_FIRMWARE_INCLUDE_HTTPS=y
CONFIG_FIRMWARE_INCLUDE_SFTP=n
CONFIG_FIRMWARE_INCLUDE_DROPBEAR=y
CONFIG_FIRMWARE_INCLUDE_DROPBEAR_FAST_CODE=y
CONFIG_FIRMWARE_INCLUDE_OPENSSH=n
CONFIG_FIRMWARE_INCLUDE_OPENVPN=n
CONFIG_FIRMWARE_INCLUDE_OPENSSL_EC=y
CONFIG_FIRMWARE_INCLUDE_OPENSSL_EXE=n
CONFIG_FIRMWARE_INCLUDE_XUPNPD=n
CONFIG_FIRMWARE_INCLUDE_MINIDLNA=n
CONFIG_FIRMWARE_INCLUDE_FIREFLY=n
CONFIG_FIRMWARE_INCLUDE_FFMPEG_NEW=n
CONFIG_FIRMWARE_INCLUDE_TRANSMISSION=n
CONFIG_FIRMWARE_INCLUDE_TRANSMISSION_WEB_CONTROL=n
CONFIG_FIRMWARE_INCLUDE_ARIA=n
CONFIG_FIRMWARE_INCLUDE_ARIA_WEB_CONTROL=n
CONFIG_FIRMWARE_INCLUDE_CURL=y
CONFIG_FIRMWARE_INCLUDE_SCUTCLIENT=n
CONFIG_FIRMWARE_INCLUDE_GDUT_DRCOM=n
CONFIG_FIRMWARE_INCLUDE_DOGCOM=n
CONFIG_FIRMWARE_INCLUDE_MINIEAP=n
CONFIG_FIRMWARE_INCLUDE_NJIT_CLIENT=n
CONFIG_FIRMWARE_INCLUDE_NAPT66=n
CONFIG_FIRMWARE_INCLUDE_SOFTETHERVPN_SERVER=n
CONFIG_FIRMWARE_INCLUDE_SOFTETHERVPN_CLIENT=n
CONFIG_FIRMWARE_INCLUDE_SOFTETHERVPN_CMD=n
CONFIG_FIRMWARE_INCLUDE_VLMCSD=n
CONFIG_FIRMWARE_INCLUDE_TTYD=y
CONFIG_FIRMWARE_INCLUDE_LRZSZ=n
CONFIG_FIRMWARE_INCLUDE_HTOP=y
CONFIG_FIRMWARE_INCLUDE_NANO=n
CONFIG_FIRMWARE_INCLUDE_IPERF3=y
CONFIG_FIRMWARE_INCLUDE_DUMP1090=n
CONFIG_FIRMWARE_INCLUDE_RTL_SDR=n
CONFIG_FIRMWARE_INCLUDE_MTR=y
CONFIG_FIRMWARE_INCLUDE_SOCAT=n
CONFIG_FIRMWARE_INCLUDE_SRELAY=n
CONFIG_FIRMWARE_INCLUDE_MENTOHUST=n
CONFIG_FIRMWARE_INCLUDE_FRPC=n
CONFIG_FIRMWARE_INCLUDE_FRPS=n
CONFIG_FIRMWARE_INCLUDE_TUNSAFE=n
CONFIG_FIRMWARE_INCLUDE_WIREGUARD=n
"""


def patch_ft_c():
    path = os.path.join(TRUNK, "proprietary/rt_wifi/rtpci/3.0.X.X/mt76x2/common/ft.c")
    with open(path, "r", encoding="utf-8", errors="replace") as f:
        txt = f.read()

    if "CONCARD_FT_PSK_LOCAL" in txt:
        print("[OK] ft.c already patched")
        return

    old_r0kh = (
        "\t\t\t\tNdisMoveMemory(pAd->ApCfg.MBSSID[Loop].FtCfg.FtR0khId, tmpbuf, strlen(tmpbuf));\n"
        "\t\t\t\tpAd->ApCfg.MBSSID[Loop].FtCfg.FtR0khId[strlen(tmpbuf)] = '\\0';"
    )
    new_r0kh = (
        "\t\t\t\tNdisZeroMemory(pAd->ApCfg.MBSSID[Loop].FtCfg.FtR0khId, sizeof(pAd->ApCfg.MBSSID[Loop].FtCfg.FtR0khId));\n"
        "\t\t\t\tNdisMoveMemory(pAd->ApCfg.MBSSID[Loop].FtCfg.FtR0khId, tmpbuf, strlen(tmpbuf));\n"
        "\t\t\t\tpAd->ApCfg.MBSSID[Loop].FtCfg.FtR0khId[strlen(tmpbuf)] = '\\0';\n"
        "\t\t\t\tpAd->ApCfg.MBSSID[Loop].FtCfg.FtR0khIdLen = strlen(tmpbuf);"
    )
    assert old_r0kh in txt, "ft.c: old_r0kh target not found"
    txt = txt.replace(old_r0kh, new_r0kh, 1)

    old_auth = """\t\tif ((pR1hkEntry == NULL) ||
\t\t\t(RTMPEqualMemory(pEntry->FT_PMK_R1_NAME, pR1hkEntry->PmkR1Name, LEN_PMK_NAME) == FALSE))
\t\t{

\t\t\t/*\tIf the requested R0KH is not reachable, the AP shall respond 
\t\t\t\tto the Authentication Request with status code 28 ("R0KH unreachable"). */
\t\t\t{\t\t\t\t\t\t\t\t\t\t\t
\t\t\t\treturn FT_STATUS_CODE_R0KH_UNREACHABLE;
\t\t\t}
\t\t}"""

    new_auth = """\t\tif ((pR1hkEntry == NULL) ||
\t\t\t(RTMPEqualMemory(pEntry->FT_PMK_R1_NAME, pR1hkEntry->PmkR1Name, LEN_PMK_NAME) == FALSE))
\t\t{
\t\t\t/* CONCARD_FT_PSK_LOCAL: Controller-less IEEE 802.11r FT-PSK key derivation */
\t\t\tMULTISSID_STRUCT *pMbss = &pAd->ApCfg.MBSSID[pEntry->apidx];
\t\t\tif ((pMbss->wdev.AuthMode == Ndis802_11AuthModeWPA2PSK) ||
\t\t\t\t(pMbss->wdev.AuthMode == Ndis802_11AuthModeWPA1PSKWPA2PSK))
\t\t\t{
\t\t\t\tUINT8 localPmkR0[LEN_PMK];
\t\t\t\tUINT8 localPmkR0Name[LEN_PMK_NAME];
\t\t\t\tUINT8 localPmkR1[LEN_PMK];
\t\t\t\tUINT8 localPmkR1Name[LEN_PMK_NAME];
\t\t\t\tUINT8 localCount = 0;
\t\t\t\tPUINT8 localAkm = WPA_ExtractSuiteFromRSNIE(pFtInfo_in->RSN_IE, pFtInfo_in->RSNIE_Len, AKM_SUITE, &localCount);
\t\t\t\tPUINT8 localCipher = WPA_ExtractSuiteFromRSNIE(pFtInfo_in->RSN_IE, pFtInfo_in->RSNIE_Len, PAIRWISE_SUITE, &localCount);
\t\t\t\tUCHAR ftPskOUI[4] = {0x00, 0x0F, 0xAC, 0x04};

\t\t\tif (localAkm != NULL && localCipher != NULL &&
\t\t\t\t\tRTMPEqualMemory(localAkm, ftPskOUI, 4))
\t\t\t\t{
\t\t\t\t\tFT_DerivePMKR0(pMbss->PMK, LEN_PMK,
\t\t\t\t\t\t(PUINT8)pMbss->Ssid, pMbss->SsidLen,
\t\t\t\t\t\tpFtCfg->FtMdId,
\t\t\t\t\t\tpFtCfg->FtR0khId, pFtCfg->FtR0khIdLen,
\t\t\t\t\t\tpEntry->Addr,
\t\t\t\t\t\tlocalPmkR0, localPmkR0Name);

\t\t\t\t\tif (RTMPEqualMemory(pPmkR0Name, localPmkR0Name, LEN_PMK_NAME) == TRUE)
\t\t\t\t\t{
\t\t\t\t\t\tUINT32 keyLife = (pMbss->PMKCachePeriod > 0) ? (pMbss->PMKCachePeriod / OS_HZ) : 43200;
\t\t\t\t\t\tFT_DerivePMKR1(localPmkR0, localPmkR0Name,
\t\t\t\t\t\t\tpMbss->wdev.bssid, pEntry->Addr,
\t\t\t\t\t\t\tlocalPmkR1, localPmkR1Name);

\t\t\t\t\t\tif (pR1hkEntry != NULL)
\t\t\t\t\t\t\tFT_R1khEntryDelete(pAd, pR1hkEntry);

\t\t\t\t\t\tNdisMoveMemory(pEntry->FT_PMK_R0, localPmkR0, LEN_PMK);
\t\t\t\t\t\tNdisMoveMemory(pEntry->FT_PMK_R0_NAME, localPmkR0Name, LEN_PMK_NAME);
\t\t\t\t\t\tFT_R1khEntryInsert(pAd,
\t\t\t\t\t\t\tlocalPmkR0Name, localPmkR1Name, localPmkR1,
\t\t\t\t\t\t\tlocalCipher, localAkm,
\t\t\t\t\t\t\tkeyLife, 20,
\t\t\t\t\t\t\tpFtCfg->FtR0khId, pFtCfg->FtR0khIdLen,
\t\t\t\t\t\t\tpEntry->Addr);
\t\t\t\t\t\tpR1hkEntry = FT_R1khEntryTabLookup(pAd, pEntry->FT_PMK_R1_NAME);
\t\t\t\t\t}
\t\t\t\t}
\t\t\t}

\t\t\t/*\tIf the requested R0KH is still not reachable, respond with status code 28. */
\t\t\tif ((pR1hkEntry == NULL) ||
\t\t\t\t(RTMPEqualMemory(pEntry->FT_PMK_R1_NAME, pR1hkEntry->PmkR1Name, LEN_PMK_NAME) == FALSE))
\t\t\t{
\t\t\t\treturn FT_STATUS_CODE_R0KH_UNREACHABLE;
\t\t\t}
\t\t}"""

    assert old_auth in txt, "ft.c: old_auth target not found"
    txt = txt.replace(old_auth, new_auth, 1)

    with open(path, "w", encoding="utf-8") as f:
        f.write(txt)
    print("[OK] Patched ft.c (FtR0khIdLen fix + CONCARD_FT_PSK_LOCAL)")


def patch_rrm_neighbor_inject():
    mt76_dirty = False
    mt76_full_clean = False

    # 0. Enable IEEE 802.11e QBSS Load Element (Tag 11) in mt76x2_ap/Makefile
    mt_mk_path = os.path.join(TRUNK, "proprietary/rt_wifi/rtpci/3.0.X.X/mt76x2_ap/Makefile")
    with open(mt_mk_path, "r", encoding="utf-8", errors="replace") as f:
        mt_mk = f.read()
    old_qload_mk = "#EXTRA_CFLAGS += -DAP_QLOAD_SUPPORT -DQLOAD_FUNC_BUSY_TIME_STATS"
    new_qload_mk = "EXTRA_CFLAGS += -DAP_QLOAD_SUPPORT -DQLOAD_FUNC_BUSY_TIME_STATS"
    if old_qload_mk in mt_mk:
        mt_mk = mt_mk.replace(old_qload_mk, new_qload_mk, 1)
        with open(mt_mk_path, "w", encoding="utf-8", newline="\n") as f:
            f.write(mt_mk)
        mt76_dirty = True
        mt76_full_clean = True
        print("[OK] Enabled -DAP_QLOAD_SUPPORT -DQLOAD_FUNC_BUSY_TIME_STATS in mt76x2_ap/Makefile")

    # 0b. Patch ap_qload.c (ED_MONITOR-safe CH_BUSY_STA sharing + overflow-safe ChanUtil + Set_QloadEnable_Proc)
    qload_c = os.path.join(TRUNK, "proprietary/rt_wifi/rtpci/3.0.X.X/mt76x2/ap/ap_qload.c")
    with open(qload_c, "r", encoding="utf-8", errors="replace") as f:
        ql_txt = f.read()
    ql_changed = False
    if "CONCARD_V21_QLOAD_ED_SAFE" not in ql_txt:
        old_q_read = "\tRTMP_IO_READ32(pAd, CH_BUSY_STA, &BusyTime);"
        new_q_read = (
            "#ifdef ED_MONITOR\n"
            "\t/* CONCARD_V21_QLOAD_ED_SAFE: share 100ms CH_BUSY_STA sample without Read-Clear contention */\n"
            "\tif (pAd->ed_chk_period > 0)\n"
            "\t\tBusyTime = pAd->ch_busy_stat[pAd->ed_stat_lidx];\n"
            "\telse\n"
            "#endif\n"
            "\tRTMP_IO_READ32(pAd, CH_BUSY_STA, &BusyTime);"
        )
        assert old_q_read in ql_txt, "ap_qload.c: CH_BUSY_STA read anchor not found"
        ql_txt = ql_txt.replace(old_q_read, new_q_read, 1)

        old_q_div = """\t\tChanUtilNu = pAd->phy_ctrl.QloadChanUtilTotal;
\t\tChanUtilNu *= 255;

\t\tChanUtilDe = pAd->phy_ctrl.QloadChanUtilBeaconInt;

\t\t/*
\t\t\tStill use pAd->CommonCfg.BeaconPeriod.
\t\t\tBecause we change QloadChanUtil not every TBTT.
\t\t*/
\t\tChanUtilDe *= pAd->CommonCfg.BeaconPeriod;

\t\tChanUtilDe <<= 10; /* ms to us */
\t\tpAd->phy_ctrl.QloadChanUtil = (UINT8)(ChanUtilNu/ChanUtilDe);
\t\tif((ChanUtilNu/ChanUtilDe) >= 255)
\t\t\tpAd->phy_ctrl.QloadChanUtil = 255;"""
        new_q_div = """\t\tChanUtilNu = (pAd->phy_ctrl.QloadChanUtilTotal >> 8) * 255U;
\t\tChanUtilDe = (pAd->phy_ctrl.QloadChanUtilBeaconInt * pAd->CommonCfg.BeaconPeriod) << 2;
\t\tif (ChanUtilDe == 0)
\t\t\tChanUtilDe = 1;
\t\tpAd->phy_ctrl.QloadChanUtil = (ChanUtilNu / ChanUtilDe >= 255U) ? 255 : (UINT8)(ChanUtilNu / ChanUtilDe);"""
        assert old_q_div in ql_txt, "ap_qload.c: ChanUtilNu division block not found"
        ql_txt = ql_txt.replace(old_q_div, new_q_div, 1)

        old_q_end = "#endif /* AP_QLOAD_SUPPORT */"
        qload_proc_fn = """
INT Set_QloadEnable_Proc(
\tIN PRTMP_ADAPTER pAd,
\tIN PSTRING Arg)
{
\tLONG en = simple_strtol(Arg, 0, 10);
\tpAd->phy_ctrl.FlgQloadEnable = (en != 0) ? TRUE : FALSE;
\treturn TRUE;
}

#endif /* AP_QLOAD_SUPPORT */"""
        assert ql_txt.rstrip().endswith(old_q_end), "ap_qload.c: trailing #endif /* AP_QLOAD_SUPPORT */ not found"
        last_idx = ql_txt.rfind(old_q_end)
        ql_txt = ql_txt[:last_idx] + qload_proc_fn + "\n"
        ql_changed = True

    if ql_changed:
        with open(qload_c, "w", encoding="utf-8", newline="\n") as f:
            f.write(ql_txt)
        mt76_dirty = True
        print("[OK] Patched ap_qload.c (ED_MONITOR-safe QBSS Load + Set_QloadEnable_Proc)")

    rrm_h = os.path.join(TRUNK, "proprietary/rt_wifi/rtpci/3.0.X.X/mt76x2/include/rrm.h")
    with open(rrm_h, "r", encoding="utf-8", errors="replace") as f:
        htxt = f.read()
    rrm_h_changed = False
    if "Set_RrmNrList_Proc" not in htxt:
        anchor_h = "INT Set_Dot11kRRM_Enable_Proc(\n\tIN PRTMP_ADAPTER pAd,\n\tIN\tPSTRING\t\targ);"
        assert anchor_h in htxt, "rrm.h: Set_Dot11kRRM_Enable_Proc anchor not found"
        htxt = htxt.replace(
            anchor_h,
            anchor_h + "\n\nINT Set_RrmNrList_Proc(\n\tIN PRTMP_ADAPTER pAd,\n\tIN\tPSTRING\t\targ);",
            1,
        )
        rrm_h_changed = True
    if "Set_BtmReq_Proc" not in htxt:
        anchor_nr_h = "INT Set_RrmNrList_Proc(\n\tIN PRTMP_ADAPTER pAd,\n\tIN\tPSTRING\t\targ);"
        assert anchor_nr_h in htxt, "rrm.h: Set_RrmNrList_Proc anchor not found"
        htxt = htxt.replace(
            anchor_nr_h,
            anchor_nr_h + "\n\nINT Set_BtmReq_Proc(\n\tIN PRTMP_ADAPTER pAd,\n\tIN\tPSTRING\t\targ);",
            1,
        )
        rrm_h_changed = True
    if rrm_h_changed:
        with open(rrm_h, "w", encoding="utf-8", newline="\n") as f:
            f.write(htxt)
        mt76_dirty = True
        print("[OK] Patched rrm.h (Set_RrmNrList_Proc + Set_BtmReq_Proc prototypes)")

    ap_cfg_c = os.path.join(TRUNK, "proprietary/rt_wifi/rtpci/3.0.X.X/mt76x2/ap/ap_cfg.c")
    with open(ap_cfg_c, "r", encoding="utf-8", errors="replace") as f:
        acfg = f.read()
    acfg_changed = False
    if '"RrmNrList"' not in acfg:
        anchor_cfg = '\t{"RrmEnable",\t\t\t\t\tSet_Dot11kRRM_Enable_Proc},'
        assert anchor_cfg in acfg, "ap_cfg.c: RrmEnable anchor not found"
        acfg = acfg.replace(
            anchor_cfg,
            anchor_cfg + '\n\t{"RrmNrList",\t\t\t\t\tSet_RrmNrList_Proc},',
            1,
        )
        acfg_changed = True
    if '"BtmReq"' not in acfg:
        anchor_btm = '\t{"RrmNrList",\t\t\t\t\tSet_RrmNrList_Proc},'
        assert anchor_btm in acfg, "ap_cfg.c: RrmNrList anchor not found"
        acfg = acfg.replace(
            anchor_btm,
            anchor_btm + '\n\t{"BtmReq",\t\t\t\t\t\tSet_BtmReq_Proc},',
            1,
        )
        acfg_changed = True
    if "Set_QloadEnable_Proc" not in acfg:
        anchor_ql_proto = "INT\tSet_QloadClr_Proc("
        assert anchor_ql_proto in acfg, "ap_cfg.c: Set_QloadClr_Proc prototype not found"
        acfg = acfg.replace(
            anchor_ql_proto,
            "INT\tSet_QloadEnable_Proc(\n\tIN\tPRTMP_ADAPTER\tpAd,\n\tIN\tPSTRING\t\t\tArg);\n\n" + anchor_ql_proto,
            1,
        )
        anchor_ql_tbl = '\t{"qloadclr",\t\t\t\t\tSet_QloadClr_Proc},'
        assert anchor_ql_tbl in acfg, "ap_cfg.c: qloadclr table entry not found"
        acfg = acfg.replace(
            anchor_ql_tbl,
            '\t{"QloadEnable",\t\t\t\t\tSet_QloadEnable_Proc},\n' + anchor_ql_tbl,
            1,
        )
        acfg_changed = True
    if "CONCARD_V2_CCA_BUSY_STAT_32" not in acfg:
        old_stat_blk = (
            '\tsprintf(msg+strlen(msg), "False CCA (one second)          = %ld\\n", (ULONG)pAd->RalinkCounters.OneSecFalseCCACnt);\n'
            "#endif /* ENHANCED_STAT_DISPLAY */"
        )
        cca_tail_32 = """\t/* CONCARD_V2_CCA_BUSY_STAT_32: Expose 1s FalseCCA & Hardware CH_BUSY_STA % (32-bit MIPS safe) */
\t{
\t\tUINT32 ch_busy_pct = 0;
#ifdef ED_MONITOR
\t\tif (pAd->ed_chk_period > 0) {
\t\t\tUINT32 period_us = pAd->ed_chk_period * 1000U;
\t\t\tUINT32 sum_busy_kb = 0;
\t\t\tUINT32 valid_cnt = 0;
\t\t\tINT bi;
\t\t\tfor (bi = 0; bi < ED_STAT_CNT; bi++) {
\t\t\t\tif (pAd->ch_busy_stat[bi] > 0 || pAd->ch_idle_stat[bi] > 0) {
\t\t\t\t\tsum_busy_kb += (pAd->ch_busy_stat[bi] >> 4);
\t\t\t\t\tvalid_cnt++;
\t\t\t\t}
\t\t\t}
\t\t\tif (valid_cnt > 0 && (period_us >> 4) > 0) {
\t\t\t\tUINT32 avg_busy_s = sum_busy_kb / valid_cnt;
\t\t\t\tch_busy_pct = (avg_busy_s * 100U) / (period_us >> 4);
\t\t\t\tif (ch_busy_pct > 100U)
\t\t\t\t\tch_busy_pct = 100U;
\t\t\t}
\t\t}
#endif
\t\tsprintf(msg+strlen(msg), "False CCA (1s)                  = %lu\\n", (ULONG)pAd->RalinkCounters.OneSecFalseCCACnt);
\t\tsprintf(msg+strlen(msg), "Channel busy                    = %u\\n", ch_busy_pct);
\t}"""
        assert old_stat_blk in acfg, "ap_cfg.c: ENHANCED_STAT_DISPLAY anchor not found"
        acfg = acfg.replace(old_stat_blk, old_stat_blk + "\n" + cca_tail_32, 1)
        acfg_changed = True
    if acfg_changed:
        with open(ap_cfg_c, "w", encoding="utf-8", newline="\n") as f:
            f.write(acfg)
        mt76_dirty = True
        print("[OK] Patched ap_cfg.c (QloadEnable + BtmReq + False CCA 1s + Hardware Channel Busy %)")

    cmm_cfg_c = os.path.join(TRUNK, "proprietary/rt_wifi/rtpci/3.0.X.X/mt76x2/common/cmm_cfg.c")
    with open(cmm_cfg_c, "r", encoding="utf-8", errors="replace") as f:
        ccfg = f.read()
    if "CONCARD_READ_CH_BUSY_STA" not in ccfg:
        old_idle = "\tRTMP_IO_READ32(pAd, CH_IDLE_STA, &ch_idle_stat);\n\tRTMP_IO_READ32(pAd, 0x1140, &ed_stat);"
        new_idle = (
            "\tRTMP_IO_READ32(pAd, CH_IDLE_STA, &ch_idle_stat);\n"
            "\tRTMP_IO_READ32(pAd, CH_BUSY_STA, &ch_busy_stat); /* CONCARD_READ_CH_BUSY_STA */\n"
            "\tRTMP_IO_READ32(pAd, 0x1140, &ed_stat);"
        )
        assert old_idle in ccfg, "cmm_cfg.c: CH_IDLE_STA anchor not found"
        ccfg = ccfg.replace(old_idle, new_idle, 1)
        with open(cmm_cfg_c, "w", encoding="utf-8", newline="\n") as f:
            f.write(ccfg)
        mt76_dirty = True
        print("[OK] Patched cmm_cfg.c (sample CH_BUSY_STA 0x1134 every 100ms)")

    ap_conn_c = os.path.join(TRUNK, "proprietary/rt_wifi/rtpci/3.0.X.X/mt76x2/ap/ap_connect.c")
    with open(ap_conn_c, "r", encoding="utf-8", errors="replace") as f:
        aconn = f.read()
    if "CONCARD_80211V_EXTCAP" not in aconn:
        anchor_ec = "#ifdef DOT11V_WNM_SUPPORT\n\t\tif (IS_BSS_TRANSIT_MANMT_SUPPORT(pAd, apidx))"
        new_ec = (
            "\t\t/* CONCARD_80211V_EXTCAP: Advertise IEEE 802.11v BSS Transition Management */\n"
            "\t\tif (pAd->ApCfg.MBSSID[apidx].RrmCfg.bDot11kRRMEnable)\n"
            "\t\t\textCapInfo.BssTransitionManmt = 1;\n"
            + anchor_ec
        )
        assert anchor_ec in aconn, "ap_connect.c: DOT11V_WNM_SUPPORT anchor not found"
        aconn = aconn.replace(anchor_ec, new_ec, 1)
        with open(ap_conn_c, "w", encoding="utf-8", newline="\n") as f:
            f.write(aconn)
        mt76_dirty = True
        print("[OK] Patched ap_connect.c (802.11v BTM Extended Capability bit 19 in Beacon/ProbeResp)")

    # Patch ap_assoc.c (802.11v AssocResp + v2.1 Zero-CPU 802.11 AssocReq Vendor IE Fingerprinting)
    ap_assoc_c = os.path.join(TRUNK, "proprietary/rt_wifi/rtpci/3.0.X.X/mt76x2/ap/ap_assoc.c")
    with open(ap_assoc_c, "r", encoding="utf-8", errors="replace") as f:
        aassoc = f.read()
    aassoc_changed = False
    if "CONCARD_80211V_ASSOC_EXTCAP" not in aassoc:
        anchor_as = "#ifdef DOT11V_WNM_SUPPORT\n\t\tif (IS_BSS_TRANSIT_MANMT_SUPPORT(pAd, pEntry->apidx))"
        new_as = (
            "\t\t/* CONCARD_80211V_ASSOC_EXTCAP: Echo 802.11v BTM support in AssocResp */\n"
            "\t\tif (pAd->ApCfg.MBSSID[pEntry->apidx].RrmCfg.bDot11kRRMEnable && ie_list->ExtCapInfo.BssTransitionManmt == 1)\n"
            "\t\t\textCapInfo.BssTransitionManmt = 1;\n"
            + anchor_as
        )
        assert anchor_as in aassoc, "ap_assoc.c: DOT11V_WNM_SUPPORT anchor not found"
        aassoc = aassoc.replace(anchor_as, new_as, 1)
        aassoc_changed = True

    if "CONCARD_V21_ASSOC_VENDOR_IE" not in aassoc:
        inc_anchor = '#include "rt_config.h"'
        assert inc_anchor in aassoc, "ap_assoc.c: #include rt_config.h not found"
        aassoc = aassoc.replace(
            inc_anchor,
            inc_anchor
            + "\n\n/* CONCARD_V21_ASSOC_VENDOR_IE: Per-WCID 802.11 AssocReq Vendor IE OS/Brand Table */\n"
            + "UINT8 g_ConCardStaVendorHint[MAX_LEN_OF_MAC_TABLE];",
            1,
        )
        mbss_anchor = "\tpMbss = &pAd->ApCfg.MBSSID[pEntry->apidx];\n\twdev = &pMbss->wdev;"
        assert mbss_anchor in aassoc, "ap_assoc.c: pMbss/wdev anchor not found"
        vendor_ie_scan = """\tpMbss = &pAd->ApCfg.MBSSID[pEntry->apidx];
\twdev = &pMbss->wdev;

\t/* CONCARD_V21_ASSOC_VENDOR_IE: Scan Tag 221 Vendor IEs on Assoc/Reassoc (~0% CPU) */
\tif (pEntry->wcid < MAX_LEN_OF_MAC_TABLE)
\t{
\t\tPFRAME_802_11 pFr = (PFRAME_802_11)Elem->Msg;
\t\tULONG ieOff = isReassoc ? 10 : 4;
\t\tUINT8 vHint = 0;
\t\tBOOLEAN hasApple = FALSE, hasSamsung = FALSE, hasMsft = FALSE, hasGoogle = FALSE, hasCnOem = FALSE;
\t\twhile ((sizeof(HEADER_802_11) + ieOff + 2) <= Elem->MsgLen)
\t\t{
\t\t\tUCHAR eid = (UCHAR)pFr->Octet[ieOff];
\t\t\tUCHAR elen = (UCHAR)pFr->Octet[ieOff + 1];
\t\t\tif ((sizeof(HEADER_802_11) + ieOff + 2 + elen) > Elem->MsgLen)
\t\t\t\tbreak;
\t\t\tif (eid == 221 && elen >= 3)
\t\t\t{
\t\t\t\tUCHAR o0 = (UCHAR)pFr->Octet[ieOff + 2];
\t\t\t\tUCHAR o1 = (UCHAR)pFr->Octet[ieOff + 3];
\t\t\t\tUCHAR o2 = (UCHAR)pFr->Octet[ieOff + 4];
\t\t\t\tUCHAR o3 = (elen >= 4) ? (UCHAR)pFr->Octet[ieOff + 5] : 0;
\t\t\t\tif ((o0 == 0x00 && o1 == 0x17 && o2 == 0xF2) ||
\t\t\t\t    (o0 == 0x00 && o1 == 0x10 && o2 == 0xFA))
\t\t\t\t\thasApple = TRUE;
\t\t\t\telse if (o0 == 0x00 && o1 == 0x00 && o2 == 0xF0)
\t\t\t\t\thasSamsung = TRUE;
\t\t\t\telse if (o0 == 0x00 && o1 == 0x1A && o2 == 0x11)
\t\t\t\t\thasGoogle = TRUE;
\t\t\t\telse if (o0 == 0x00 && o1 == 0x50 && o2 == 0xF2 &&
\t\t\t\t         o3 != 0x01 && o3 != 0x02 && o3 != 0x04)
\t\t\t\t\thasMsft = TRUE;
\t\t\t\telse if ((o0 == 0x00 && o1 == 0xE0 && o2 == 0xFC) ||
\t\t\t\t         (o0 == 0x8C && o1 == 0xBE && o2 == 0xBE))
\t\t\t\t\thasCnOem = TRUE;
\t\t\t}
\t\t\tieOff += (2 + elen);
\t\t}
\t\tif (hasApple)
\t\t\tvHint = (ie_list->HTCapability.MCSSet[2] != 0) ? 6 : 1;
\t\telse if (hasSamsung)
\t\t\tvHint = 2;
\t\telse if (hasGoogle)
\t\t\tvHint = 4;
\t\telse if (hasCnOem)
\t\t\tvHint = 5;
\t\telse if (hasMsft)
\t\t\tvHint = 3;
\t\tg_ConCardStaVendorHint[pEntry->wcid] = vHint;
\t}"""
        aassoc = aassoc.replace(mbss_anchor, vendor_ie_scan, 1)
        aassoc_changed = True

    if aassoc_changed:
        with open(ap_assoc_c, "w", encoding="utf-8", newline="\n") as f:
            f.write(aassoc)
        mt76_dirty = True
        print("[OK] Patched ap_assoc.c (802.11v AssocResp + CONCARD_V21_ASSOC_VENDOR_IE)")

    # Patch cmm_info.c (export Vendor IE code in bits 16..23 of LastRxRate without changing ABI size)
    cmm_info_c = os.path.join(TRUNK, "proprietary/rt_wifi/rtpci/3.0.X.X/mt76x2/common/cmm_info.c")
    with open(cmm_info_c, "r", encoding="utf-8", errors="replace") as f:
        cinfo = f.read()
    if "CONCARD_V21_PACK_VENDOR_HINT" not in cinfo:
        old_cme = (
            "static VOID\n"
            "copy_mac_table_entry(RT_802_11_MAC_ENTRY *pDst, MAC_TABLE_ENTRY *pEntry)"
        )
        assert old_cme in cinfo, "cmm_info.c: copy_mac_table_entry anchor not found"
        cinfo = cinfo.replace(
            old_cme,
            "extern UINT8 g_ConCardStaVendorHint[];\n\n" + old_cme,
            1,
        )
        old_lrx = "\tpDst->TxRate.word = pEntry->HTPhyMode.word;\n\tpDst->LastRxRate = pEntry->LastRxRate;"
        new_lrx = (
            "\tpDst->TxRate.word = pEntry->HTPhyMode.word;\n"
            "\t/* CONCARD_V21_PACK_VENDOR_HINT: bits 16..23 carry Assoc Vendor IE OS code */\n"
            "\tpDst->LastRxRate = (pEntry->LastRxRate & 0xFFFFU) |\n"
            "\t\t(((UINT32)((pEntry->wcid < MAX_LEN_OF_MAC_TABLE) ? g_ConCardStaVendorHint[pEntry->wcid] : 0)) << 16);"
        )
        assert old_lrx in cinfo, "cmm_info.c: LastRxRate assignment not found"
        cinfo = cinfo.replace(old_lrx, new_lrx, 1)
        with open(cmm_info_c, "w", encoding="utf-8", newline="\n") as f:
            f.write(cinfo)
        mt76_dirty = True
        print("[OK] Patched cmm_info.c (CONCARD_V21_PACK_VENDOR_HINT in LastRxRate[23:16])")

    rrm_tlv_c = os.path.join(TRUNK, "proprietary/rt_wifi/rtpci/3.0.X.X/mt76x2/common/rrm_tlv.c")
    with open(rrm_tlv_c, "r", encoding="utf-8", errors="replace") as f:
        tlv = f.read()
    tlv_changed = False
    if "CONCARD_CONTROLLER_RRM_NR" not in tlv:
        anchor_fn = "VOID RRM_EnqueueNeighborRep("
        assert anchor_fn in tlv, "rrm_tlv.c: RRM_EnqueueNeighborRep not found"
        nr_helper = """/* CONCARD_CONTROLLER_RRM_NR: Controller-pushed zero-scan 802.11k Neighbor Report table */
typedef struct _CONCARD_RRM_NR_ITEM {
\tUCHAR Bssid[MAC_ADDR_LEN];
\tUINT8 Channel;
} CONCARD_RRM_NR_ITEM;

static CONCARD_RRM_NR_ITEM g_ConCardNrTab[16];
static INT g_ConCardNrCnt = 0;

static INT ConCard_HexNib(CHAR c)
{
\tif (c >= '0' && c <= '9') return c - '0';
\tif (c >= 'a' && c <= 'f') return c - 'a' + 10;
\tif (c >= 'A' && c <= 'F') return c - 'A' + 10;
\treturn -1;
}

INT Set_RrmNrList_Proc(
\tIN PRTMP_ADAPTER pAd,
\tIN PSTRING arg)
{
\tCONCARD_RRM_NR_ITEM tmpTab[16];
\tINT tmpCnt = 0;
\tPSTRING pCur;

\tif (arg == NULL || arg[0] == '\\0' || strcmp(arg, "clear") == 0 || strcmp(arg, "0") == 0)
\t{
\t\tg_ConCardNrCnt = 0;
\t\treturn TRUE;
\t}

\tNdisZeroMemory(tmpTab, sizeof(tmpTab));
\tpCur = arg;
\twhile (*pCur != '\\0' && tmpCnt < 16)
\t{
\t\tUCHAR macBytes[6];
\t\tINT nibCnt = 0;
\t\tINT hiNib = -1;
\t\tLONG chVal = 0;

\t\twhile (*pCur == ';' || *pCur == ' ' || *pCur == '\\t')
\t\t\tpCur++;
\t\tif (*pCur == '\\0')
\t\t\tbreak;

\t\twhile (*pCur != '\\0' && *pCur != ';' && nibCnt < 12)
\t\t{
\t\t\tINT v;
\t\t\tif (*pCur == ':' || *pCur == '-')
\t\t\t{
\t\t\t\tpCur++;
\t\t\t\tcontinue;
\t\t\t}
\t\t\tv = ConCard_HexNib(*pCur);
\t\t\tif (v < 0)
\t\t\t\tbreak;
\t\t\tif ((nibCnt & 1) == 0)
\t\t\t\thiNib = v;
\t\t\telse
\t\t\t\tmacBytes[nibCnt >> 1] = (UCHAR)((hiNib << 4) | v);
\t\t\tnibCnt++;
\t\t\tpCur++;
\t\t}

\t\tif (nibCnt == 12 && (*pCur == ':' || *pCur == ','))
\t\t{
\t\t\tpCur++;
\t\t\tchVal = simple_strtol(pCur, 0, 10);
\t\t\tif (chVal >= 1 && chVal <= 13)
\t\t\t{
\t\t\t\tNdisMoveMemory(tmpTab[tmpCnt].Bssid, macBytes, MAC_ADDR_LEN);
\t\t\t\ttmpTab[tmpCnt].Channel = (UINT8)chVal;
\t\t\t\ttmpCnt++;
\t\t\t}
\t\t}

\t\twhile (*pCur != '\\0' && *pCur != ';')
\t\t\tpCur++;
\t}

\tNdisMoveMemory(g_ConCardNrTab, tmpTab, sizeof(tmpTab));
\tg_ConCardNrCnt = tmpCnt;
\treturn TRUE;
}

"""
        tlv = tlv.replace(anchor_fn, nr_helper + anchor_fn, 1)

        old_scan_loop = "#ifdef AP_SCAN_SUPPORT\n\t/* insert NeighborRep IE. */\n\tfor (loop = 0; loop < pAd->ScanTab.BssNr; loop++)"
        new_scan_loop = """\tif (g_ConCardNrCnt > 0)
\t{
\t\tfor (loop = 0; loop < g_ConCardNrCnt; loop++)
\t\t{
\t\t\tRRM_BSSID_INFO BssidInfo;
\t\t\tPktLen = FrameLen + sizeof(RRM_NEIGHBOR_REP_INFO);
\t\t\tif (PktLen >= MGMT_DMA_BUFFER_SIZE)
\t\t\t\tbreak;
\t\t\tif (MAC_ADDR_EQUAL(g_ConCardNrTab[loop].Bssid, pAd->ApCfg.MBSSID[pEntry->apidx].wdev.bssid))
\t\t\t\tcontinue;
\t\t\tBssidInfo.word = 0;
\t\t\tBssidInfo.field.APReachAble = 3;
\t\t\tBssidInfo.field.Security = 1;
\t\t\tBssidInfo.field.KeyScope = 1;
\t\t\tBssidInfo.field.Qos = 1;
\t\t\tBssidInfo.field.APSD = 1;
\t\t\tBssidInfo.field.RRM = 1;
\t\t\tBssidInfo.field.ImmediateBA = 1;
#ifdef DOT11R_FT_SUPPORT
\t\t\tBssidInfo.field.MobilityDomain = pAd->ApCfg.MBSSID[pEntry->apidx].FtCfg.FtCapFlag.Dot11rFtEnable ? 1 : 0;
#endif
\t\t\tBssidInfo.field.HT = 1;
\t\t\tRRM_InsertNeighborRepIE(pAd, (pOutBuffer + FrameLen), &FrameLen,
\t\t\t\tsizeof(RRM_NEIGHBOR_REP_INFO), g_ConCardNrTab[loop].Bssid,
\t\t\t\tBssidInfo, 81, g_ConCardNrTab[loop].Channel, 7);
\t\t}
\t}
#ifdef AP_SCAN_SUPPORT
\t/* insert NeighborRep IE. */
\tfor (loop = 0; g_ConCardNrCnt == 0 && loop < pAd->ScanTab.BssNr; loop++)"""
        assert old_scan_loop in tlv, "rrm_tlv.c: old_scan_loop anchor not found"
        tlv = tlv.replace(old_scan_loop, new_scan_loop, 1)
        tlv_changed = True

    if "CONCARD_80211V_BTM_REQ" not in tlv:
        anchor_fn2 = "VOID RRM_EnqueueNeighborRep("
        assert anchor_fn2 in tlv, "rrm_tlv.c: RRM_EnqueueNeighborRep not found for BTM"
        btm_helper = """/* CONCARD_80211V_BTM_REQ: Soft Non-Dropping IEEE 802.11v WNM BSS Transition Management Request */
static UINT8 g_ConCardBtmToken = 1;

static PSTRING ConCard_ParseMacBytes(PSTRING pStr, UCHAR outMac[MAC_ADDR_LEN])
{
\tINT nibCnt = 0, hiNib = -1;
\twhile (*pStr == ' ' || *pStr == '\\t' || *pStr == ',' || *pStr == ';')
\t\tpStr++;
\twhile (*pStr != '\\0' && nibCnt < 12)
\t{
\t\tINT v;
\t\tif (*pStr == ':' || *pStr == '-')
\t\t{
\t\t\tpStr++;
\t\t\tcontinue;
\t\t}
\t\tv = ConCard_HexNib(*pStr);
\t\tif (v < 0)
\t\t\tbreak;
\t\tif ((nibCnt & 1) == 0)
\t\t\thiNib = v;
\t\telse
\t\t\toutMac[nibCnt >> 1] = (UCHAR)((hiNib << 4) | v);
\t\tnibCnt++;
\t\tpStr++;
\t}
\treturn (nibCnt == 12) ? pStr : NULL;
}

INT Set_BtmReq_Proc(
\tIN PRTMP_ADAPTER pAd,
\tIN PSTRING arg)
{
\tUCHAR staMac[MAC_ADDR_LEN];
\tUCHAR targetBssid[MAC_ADDR_LEN];
\tUINT8 targetCh = 0;
\tBOOLEAN hasTarget = FALSE;
\tPSTRING pNext;
\tMAC_TABLE_ENTRY *pEntry;
\tCONCARD_RRM_NR_ITEM cands[8];
\tUINT8 candPref[8];
\tINT candCnt = 0, i;
\tPUCHAR pOutBuffer = NULL;
\tNDIS_STATUS NStatus;
\tULONG FrameLen = 0;
\tHEADER_802_11 ActHdr;
\tUCHAR hdrBytes[7];

\tif (arg == NULL || arg[0] == '\\0')
\t\treturn FALSE;

\tpNext = ConCard_ParseMacBytes(arg, staMac);
\tif (pNext == NULL)
\t\treturn FALSE;

\tpEntry = MacTableLookup(pAd, staMac);
\tif (pEntry == NULL || pEntry->apidx >= pAd->ApCfg.BssidNum)
\t\treturn FALSE;

\tif (*pNext == ',' || *pNext == ';' || *pNext == ' ')
\t{
\t\tPSTRING pAfterBssid = ConCard_ParseMacBytes(pNext + 1, targetBssid);
\t\tif (pAfterBssid != NULL)
\t\t{
\t\t\tLONG chParsed = 0;
\t\t\tif (*pAfterBssid == ':' || *pAfterBssid == ',')
\t\t\t\tchParsed = simple_strtol(pAfterBssid + 1, 0, 10);
\t\t\tif (chParsed < 1 || chParsed > 13)
\t\t\t{
\t\t\t\tfor (i = 0; i < g_ConCardNrCnt; i++)
\t\t\t\t{
\t\t\t\t\tif (MAC_ADDR_EQUAL(g_ConCardNrTab[i].Bssid, targetBssid))
\t\t\t\t\t{
\t\t\t\t\t\tchParsed = g_ConCardNrTab[i].Channel;
\t\t\t\t\t\tbreak;
\t\t\t\t\t}
\t\t\t\t}
\t\t\t}
\t\t\tif (chParsed >= 1 && chParsed <= 13)
\t\t\t{
\t\t\t\ttargetCh = (UINT8)chParsed;
\t\t\t\thasTarget = TRUE;
\t\t\t}
\t\t}
\t}

\tif (hasTarget)
\t{
\t\tNdisMoveMemory(cands[0].Bssid, targetBssid, MAC_ADDR_LEN);
\t\tcands[0].Channel = targetCh;
\t\tcandPref[0] = 255;
\t\tcandCnt = 1;
\t}

\tfor (i = 0; i < g_ConCardNrCnt && candCnt < 6; i++)
\t{
\t\tif (MAC_ADDR_EQUAL(g_ConCardNrTab[i].Bssid, pAd->ApCfg.MBSSID[pEntry->apidx].wdev.bssid))
\t\t\tcontinue;
\t\tif (hasTarget && MAC_ADDR_EQUAL(g_ConCardNrTab[i].Bssid, targetBssid))
\t\t\tcontinue;
\t\tNdisMoveMemory(cands[candCnt].Bssid, g_ConCardNrTab[i].Bssid, MAC_ADDR_LEN);
\t\tcands[candCnt].Channel = g_ConCardNrTab[i].Channel;
\t\tcandPref[candCnt] = (UINT8)(200 - candCnt * 10);
\t\tcandCnt++;
\t}

\tNStatus = MlmeAllocateMemory(pAd, (PVOID)&pOutBuffer);
\tif (NStatus != NDIS_STATUS_SUCCESS || pOutBuffer == NULL)
\t\treturn FALSE;

\tActHeaderInit(pAd, &ActHdr, pEntry->Addr,
\t\tpAd->ApCfg.MBSSID[pEntry->apidx].wdev.bssid,
\t\tpAd->ApCfg.MBSSID[pEntry->apidx].wdev.bssid);
\tNdisMoveMemory(pOutBuffer, &ActHdr, sizeof(HEADER_802_11));
\tFrameLen = sizeof(HEADER_802_11);

\tg_ConCardBtmToken++;
\tif (g_ConCardBtmToken == 0)
\t\tg_ConCardBtmToken = 1;

\thdrBytes[0] = 10; /* Category: WNM (10) */
\thdrBytes[1] = 7;  /* Action: BSS Transition Management Request (7) */
\thdrBytes[2] = g_ConCardBtmToken;
\thdrBytes[3] = (candCnt > 0) ? 0x01 : 0x00;
\thdrBytes[4] = 0;
\thdrBytes[5] = 0;
\thdrBytes[6] = 50;
\tNdisMoveMemory(pOutBuffer + FrameLen, hdrBytes, sizeof(hdrBytes));
\tFrameLen += sizeof(hdrBytes);

\tfor (i = 0; i < candCnt; i++)
\t{
\t\tUCHAR nrIe[18];
\t\tUINT32 bssidInfoWord = 0x00000EFBUL;
#ifdef DOT11R_FT_SUPPORT
\t\tif (pAd->ApCfg.MBSSID[pEntry->apidx].FtCfg.FtCapFlag.Dot11rFtEnable)
\t\t\tbssidInfoWord |= (1UL << 10);
#endif
\t\tif (FrameLen + sizeof(nrIe) >= MGMT_DMA_BUFFER_SIZE)
\t\t\tbreak;
\t\tnrIe[0] = 52;
\t\tnrIe[1] = 16;
\t\tNdisMoveMemory(&nrIe[2], cands[i].Bssid, MAC_ADDR_LEN);
\t\tnrIe[8]  = (UCHAR)(bssidInfoWord & 0xFF);
\t\tnrIe[9]  = (UCHAR)((bssidInfoWord >> 8) & 0xFF);
\t\tnrIe[10] = (UCHAR)((bssidInfoWord >> 16) & 0xFF);
\t\tnrIe[11] = (UCHAR)((bssidInfoWord >> 24) & 0xFF);
\t\tnrIe[12] = 81;
\t\tnrIe[13] = cands[i].Channel;
\t\tnrIe[14] = 7;
\t\tnrIe[15] = 3;
\t\tnrIe[16] = 1;
\t\tnrIe[17] = candPref[i];
\t\tNdisMoveMemory(pOutBuffer + FrameLen, nrIe, sizeof(nrIe));
\t\tFrameLen += sizeof(nrIe);
\t}

\tMiniportMMRequest(pAd, QID_AC_BE, pOutBuffer, FrameLen);
\tMlmeFreeMemory(pAd, pOutBuffer);
\treturn TRUE;
}

"""
        tlv = tlv.replace(anchor_fn2, btm_helper + anchor_fn2, 1)
        tlv_changed = True

    if tlv_changed:
        with open(rrm_tlv_c, "w", encoding="utf-8", newline="\n") as f:
            f.write(tlv)
        mt76_dirty = True
        print("[OK] Patched rrm_tlv.c (zero-scan 802.11k NR + 802.11v Set_BtmReq_Proc)")

    if mt76_dirty:
        drv_build = os.path.join(TRUNK, "linux-3.4.x/drivers/net/wireless/ralink")
        if os.path.isdir(drv_build):
            target_objs = {
                "rrm_tlv.o", "cmm_cfg.o", "cmm_info.o", "ap_cfg.o",
                "ap_connect.o", "ap_assoc.o", "ap_qload.o", "ap_sync.o",
                "mt76x2_ap.o", "mt76x2_ap.ko"
            }
            for rdir, _, fnames in os.walk(drv_build):
                for fn in fnames:
                    if (mt76_full_clean and fn.endswith((".o", ".ko"))) or (fn in target_objs):
                        try:
                            os.remove(os.path.join(rdir, fn))
                        except OSError:
                            pass


def patch_ralink_c():
    path = os.path.join(TRUNK, "user/rc/ralink.c")
    with open(path, "r", encoding="utf-8", errors="replace") as f:
        txt = f.read()

    changed = False
    if "CONCARD_80211KR_CFG" not in txt:
        old_rrm = '\tfprintf(fp, "RRMEnable=%d\\n", 0);\n\tfprintf(fp, "MboSupport=%d\\n", 0);'
        new_rrm = """\t/* CONCARD_80211KR_CFG: IEEE 802.11k RRM & IEEE 802.11r Fast BSS Transition */
\t{
\t\tint i_rrm = (nvram_wlan_get_int(is_aband, "rrm_enable") > 0) ? 1 : 0;
\t\tint i_ft_cfg = (nvram_wlan_get_int(is_aband, "ft_enable") > 0) ? 1 : 0;
\t\tconst char *s_auth = nvram_wlan_get(is_aband, "auth_mode");
\t\tconst char *s_crypto = nvram_wlan_get(is_aband, "crypto");
\t\tint i_wpa_m = nvram_wlan_get_int(is_aband, "wpa_mode");
\t\tint i_ft = (i_ft_cfg && !strcmp(s_auth, "psk") && i_wpa_m != 1 && strcmp(s_crypto, "tkip")) ? 1 : 0;
\t\tconst char *s_mdid = nvram_wlan_get(is_aband, "ft_mdid");
\t\tconst char *s_r0kh = nvram_wlan_get(is_aband, "ft_r0kh_id");
\t\tif (!s_mdid || strlen(s_mdid) != 2)
\t\t\ts_mdid = "CC";
\t\tif (!s_r0kh || strlen(s_r0kh) < 1 || strlen(s_r0kh) > 48)
\t\t\ts_r0kh = "ConCardFT";

\t\tfprintf(fp, "RRMEnable=%d;%d\\n", i_rrm, i_rrm);
\t\tif (!is_aband)
\t\t\tfprintf(fp, "RegulatoryClass=%d;%d\\n", 81, 81);
\t\tfprintf(fp, "FtSupport=%d;0\\n", i_ft);
\t\tfprintf(fp, "FtRic=%d;0\\n", i_ft);
\t\tfprintf(fp, "FtOtd=0;0\\n");
\t\tfprintf(fp, "FtMdId1=%s\\n", s_mdid);
\t\tfprintf(fp, "FtMdId2=%s\\n", s_mdid);
\t\tfprintf(fp, "FtR0khId1=%s\\n", s_r0kh);
\t\tfprintf(fp, "FtR0khId2=%s\\n", s_r0kh);
\t}
\tfprintf(fp, "MboSupport=%d\\n", 0);"""
        assert old_rrm in txt, "ralink.c: old_rrm target not found"
        txt = txt.replace(old_rrm, new_rrm, 1)

        old_pmk = '\tfprintf(fp, "PMKCachePeriod=%d\\n", 10);'
        new_pmk = '\tfprintf(fp, "PMKCachePeriod=%d\\n", (nvram_wlan_get_int(is_aband, "ft_enable") > 0) ? 720 : 10);'
        assert old_pmk in txt, "ralink.c: old_pmk target not found"
        txt = txt.replace(old_pmk, new_pmk, 1)
        changed = True

    if "CONCARD_PURE_OFDM_BASICRATE" not in txt:
        old_br = (
            "\t\tcase PHY_11G:\n"
            "\t\tcase PHY_11GN_MIXED:\n"
            "\t\t\ti_val = 351; // 1, 2, 5.5, 11, 6, 12, 24 Mbps\n"
            "\t\t\tbreak;"
        )
        new_br = (
            "\t\tcase PHY_11G:\n"
            "\t\tcase PHY_11GN_MIXED:\n"
            "\t\tcase PHY_11N:\n"
            "\t\t\ti_val = 336; // CONCARD_PURE_OFDM_BASICRATE: 6, 12, 24 Mbps (no 802.11b CCK drag)\n"
            "\t\t\tbreak;\n"
            "\t\tcase PHY_11BG_MIXED:\n"
            "\t\tcase PHY_11BGN_MIXED:\n"
            "\t\t\ti_val = 351; // 1, 2, 5.5, 11, 6, 12, 24 Mbps\n"
            "\t\t\tbreak;"
        )
        assert old_br in txt, "ralink.c: BasicRate PHY_11GN_MIXED anchor not found"
        txt = txt.replace(old_br, new_br, 1)
        changed = True

    if changed:
        with open(path, "w", encoding="utf-8", newline="\n") as f:
            f.write(txt)
        ralink_o = os.path.join(TRUNK, "user/rc/ralink.o")
        if os.path.exists(ralink_o):
            os.remove(ralink_o)
        print("[OK] Patched user/rc/ralink.c (802.11k/r + 12h PMK + Pure OFDM BasicRate=336)")
    else:
        print("[OK] ralink.c already patched")


def patch_defaults():
    dh_path = os.path.join(TRUNK, "user/shared/defaults.h")
    with open(dh_path, "r", encoding="utf-8", errors="replace") as f:
        dh = f.read()

    if "BOARD_N9V1" not in dh:
        old_blk = """#define DEF_LAN_ADDR\t\t"192.168.2.1"
#define DEF_LAN_DHCP_BEG\t"192.168.2.100"
#define DEF_LAN_DHCP_END\t"192.168.2.244"
#define DEF_LAN_MASK\t\t"255.255.255.0"

#define DEF_WLAN_2G_CC\t\t"CN"
#define DEF_WLAN_5G_CC\t\t"US"
#define DEF_WLAN_2G_SSID\tBOARD_PID "_%s"
#define DEF_WLAN_5G_SSID\tBOARD_PID "_5G_%s"
#define DEF_WLAN_2G_GSSID\tBOARD_PID "_GUEST_%s"
#define DEF_WLAN_5G_GSSID\tBOARD_PID "_GUEST_5G_%s"
#define DEF_WLAN_2G_PSK\t\t"1234567890"
#define DEF_WLAN_5G_PSK\t\t"1234567890"

#define DEF_ROOT_PASSWORD\t"admin"
#define DEF_SMB_WORKGROUP\t"WORKGROUP"
#define DEF_TIMEZONE\t\t"CST-8"
#define DEF_NTP_SERVER0\t\t"ntp1.aliyun.com\""""

        new_blk = """#ifdef BOARD_N9V1
#define DEF_LAN_ADDR\t\t"192.168.1.165"
#define DEF_LAN_DHCP_BEG\t"192.168.1.100"
#define DEF_LAN_DHCP_END\t"192.168.1.244"
#define DEF_LAN_MASK\t\t"255.255.255.0"

#define DEF_WLAN_2G_CC\t\t"GB"
#define DEF_WLAN_5G_CC\t\t"US"
#define DEF_WLAN_2G_SSID\t"ConCard-N9_%s"
#define DEF_WLAN_5G_SSID\t"ConCard-N9_5G_%s"
#define DEF_WLAN_2G_GSSID\t"ConCard-Guest_%s"
#define DEF_WLAN_5G_GSSID\t"ConCard-Guest_5G_%s"
#define DEF_WLAN_2G_PSK\t\t"1234567890"
#define DEF_WLAN_5G_PSK\t\t"1234567890"

#define DEF_ROOT_PASSWORD\t"admin"
#define DEF_SMB_WORKGROUP\t"WORKGROUP"
#define DEF_TIMEZONE\t\t"UCT-7"
#define DEF_NTP_SERVER0\t\t"time.google.com"
#else
#define DEF_LAN_ADDR\t\t"192.168.2.1"
#define DEF_LAN_DHCP_BEG\t"192.168.2.100"
#define DEF_LAN_DHCP_END\t"192.168.2.244"
#define DEF_LAN_MASK\t\t"255.255.255.0"

#define DEF_WLAN_2G_CC\t\t"CN"
#define DEF_WLAN_5G_CC\t\t"US"
#define DEF_WLAN_2G_SSID\tBOARD_PID "_%s"
#define DEF_WLAN_5G_SSID\tBOARD_PID "_5G_%s"
#define DEF_WLAN_2G_GSSID\tBOARD_PID "_GUEST_%s"
#define DEF_WLAN_5G_GSSID\tBOARD_PID "_GUEST_5G_%s"
#define DEF_WLAN_2G_PSK\t\t"1234567890"
#define DEF_WLAN_5G_PSK\t\t"1234567890"

#define DEF_ROOT_PASSWORD\t"admin"
#define DEF_SMB_WORKGROUP\t"WORKGROUP"
#define DEF_TIMEZONE\t\t"CST-8"
#define DEF_NTP_SERVER0\t\t"ntp1.aliyun.com"
#endif"""
        assert old_blk in dh, "defaults.h: target block not found"
        dh = dh.replace(old_blk, new_blk, 1)
        with open(dh_path, "w", encoding="utf-8") as f:
            f.write(dh)
        print("[OK] Patched user/shared/defaults.h")

    dc_path = os.path.join(TRUNK, "user/shared/defaults.c")
    with open(dc_path, "r", encoding="utf-8", errors="replace") as f:
        dc = f.read()

    dc_changed = False
    v21_defaults_blk = (
        "#ifdef BOARD_N9V1\n"
        '\t{ "rt_KickStaRssiLow", "0" },\n'
        '\t{ "rt_AssocReqRssiThres", "-84" },\n'
        '\t{ "rt_rrm_enable", "1" },\n'
        '\t{ "rt_ft_enable", "1" },\n'
        '\t{ "rt_ft_mdid", "CC" },\n'
        '\t{ "rt_ft_r0kh_id", "ConCardFT" },\n'
        '\t{ "cc_cell_breathing", "1" },\n'
        '\t{ "cc_qload_enable", "1" },\n'
        '\t{ "cc_probe_rssi", "-82" },\n'
        '\t{ "cc_fair_queue", "1" },\n'
        '\t{ "cc_l2_shield", "1" },\n'
        '\t{ "cc_led_mode", "0" },\n'
        '\t{ "cc_ctrl_url", "http://192.168.1.102:9100" },\n'
        '\t{ "cc_ota_repo_url", "https://github.com/fbaduongth/totolink_n9_firmware_custom" },\n'
        "#else\n"
        '\t{ "rt_KickStaRssiLow", "0" },\n'
        '\t{ "rt_AssocReqRssiThres", "0" },\n'
        '\t{ "rt_rrm_enable", "0" },\n'
        '\t{ "rt_ft_enable", "0" },\n'
        '\t{ "rt_ft_mdid", "CC" },\n'
        '\t{ "rt_ft_r0kh_id", "ConCardFT" },\n'
        '\t{ "cc_cell_breathing", "0" },\n'
        '\t{ "cc_qload_enable", "0" },\n'
        '\t{ "cc_probe_rssi", "0" },\n'
        '\t{ "cc_fair_queue", "0" },\n'
        '\t{ "cc_l2_shield", "0" },\n'
        '\t{ "cc_led_mode", "0" },\n'
        '\t{ "cc_ctrl_url", "" },\n'
        '\t{ "cc_ota_repo_url", "" },\n'
        "#endif"
    )

    if "rt_ft_enable" not in dc:
        old_stp = '\t{ "lan_stp", "1" },\t\t\t/* LAN spanning tree protocol */'
        new_stp = (
            "#ifdef BOARD_N9V1\n"
            '\t{ "lan_stp", "0" },\t\t\t/* Instant bridge forwarding on pure AP */\n'
            "#else\n"
            '\t{ "lan_stp", "1" },\t\t\t/* LAN spanning tree protocol */\n'
            "#endif"
        )
        assert old_stp in dc, "defaults.c: old_stp not found"
        dc = dc.replace(old_stp, new_stp, 1)

        old_bw = '\t{ "rt_HT_BW", "1" },'
        new_bw = (
            "#ifdef BOARD_N9V1\n"
            '\t{ "rt_HT_BW", "0" },\t\t\t/* 20MHz high-density enterprise channel width */\n'
            "#else\n"
            '\t{ "rt_HT_BW", "1" },\n'
            "#endif"
        )
        assert old_bw in dc, "defaults.c: old_bw not found"
        dc = dc.replace(old_bw, new_bw, 1)

        old_rssi = '\t{ "rt_HT_AMSDU", "0" },\n\t{ "rt_HT_MpduDensity", "5" },'
        new_rssi_amsdu = (
            "#ifdef BOARD_N9V1\n"
            '\t{ "rt_HT_AMSDU", "1" },\n'
            "#else\n"
            '\t{ "rt_HT_AMSDU", "0" },\n'
            "#endif\n"
            '\t{ "rt_HT_MpduDensity", "5" },'
        )
        assert old_rssi in dc, "defaults.c: old_rssi_amsdu not found"
        dc = dc.replace(old_rssi, new_rssi_amsdu, 1)

        old_kick = '\t{ "rt_KickStaRssiLow", "0" },\n\t{ "rt_AssocReqRssiThres", "0" },'
        assert old_kick in dc, "defaults.c: old_kick not found"
        dc = dc.replace(old_kick, v21_defaults_blk, 1)
        dc_changed = True
    elif "cc_ota_repo_url" not in dc:
        old_v20_blk = (
            "#ifdef BOARD_N9V1\n"
            '\t{ "rt_KickStaRssiLow", "0" },\n'
            '\t{ "rt_AssocReqRssiThres", "-84" },\n'
            '\t{ "rt_rrm_enable", "1" },\n'
            '\t{ "rt_ft_enable", "1" },\n'
            '\t{ "rt_ft_mdid", "CC" },\n'
            '\t{ "rt_ft_r0kh_id", "ConCardFT" },\n'
            '\t{ "cc_cell_breathing", "1" },\n'
            '\t{ "cc_ctrl_url", "http://192.168.1.102:9100" },\n'
            "#else\n"
            '\t{ "rt_KickStaRssiLow", "0" },\n'
            '\t{ "rt_AssocReqRssiThres", "0" },\n'
            '\t{ "rt_rrm_enable", "0" },\n'
            '\t{ "rt_ft_enable", "0" },\n'
            '\t{ "rt_ft_mdid", "CC" },\n'
            '\t{ "rt_ft_r0kh_id", "ConCardFT" },\n'
            '\t{ "cc_cell_breathing", "0" },\n'
            '\t{ "cc_ctrl_url", "" },\n'
            "#endif"
        )
        assert old_v20_blk in dc, "defaults.c: old_v20_blk not found"
        dc = dc.replace(old_v20_blk, v21_defaults_blk, 1)
        dc_changed = True

    if "#if defined(BOARD_ROLE_REPEATER)\n\t{ \"sw_mode\", \"3\" }," in dc:
        dc = dc.replace(
            "#if defined(BOARD_ROLE_REPEATER)\n\t{ \"sw_mode\", \"3\" },",
            "#if defined(BOARD_ROLE_REPEATER) || defined(BOARD_N9V1)\n\t{ \"sw_mode\", \"3\" },",
            1,
        )
        dc_changed = True

    if '\t{ "preferred_lang", "EN" },' not in dc:
        old_lang = '\t{ "preferred_lang", "CN" },'
        new_lang = (
            "#ifdef BOARD_N9V1\n"
            '\t{ "preferred_lang", "EN" },\n'
            "#else\n"
            '\t{ "preferred_lang", "CN" },\n'
            "#endif"
        )
        assert old_lang in dc, "defaults.c: preferred_lang not found"
        dc = dc.replace(old_lang, new_lang, 1)
        dc_changed = True

    if dc_changed:
        with open(dc_path, "w", encoding="utf-8", newline="\n") as f:
            f.write(dc)
        def_o = os.path.join(TRUNK, "user/shared/defaults.o")
        if os.path.exists(def_o):
            os.remove(def_o)
        print("[OK] Patched user/shared/defaults.c (Concard N9 v2.1 defaults + GitHub OTA repo)")
    else:
        print("[OK] defaults.c already patched for Concard N9 v2.1")


def patch_detect_link_c():
    dl_path = os.path.join(TRUNK, "user/rc/detect_link.c")
    with open(dl_path, "r", encoding="utf-8", errors="replace") as f:
        dl = f.read()

    if "CONCARD_V21_AP_WATCHDOG_LED" in dl:
        print("[OK] user/rc/detect_link.c already patched for Concard N9 v2.1")
        return

    anchor_reset = "static void\ndl_reset_state(void)"
    assert anchor_reset in dl, "detect_link.c: dl_reset_state anchor not found"

    v21_helpers = """#include <stdint.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>

/* CONCARD_V21_AP_WATCHDOG_LED: L1/L2/L3 Gateway ICMP+ARP Watchdog, Smart DHCP & Ceiling Ring LED */
static unsigned int dl_ap_port_mask_prev = 0;
static unsigned long dl_ap_last_dhcp_tick = 0;
static int dl_ap_gw_miss_streak = 0;
static int dl_ap_gw_ok = 1;
static int dl_ap_gw_ok_prev = -1;
static int dl_ap_led_fault_prev = -1;
static int dl_ap_led_mode_prev = -1;
static int dl_ap_icmp_sock = -1;
static uint16_t dl_ap_icmp_seq = 1;
static uint32_t dl_ap_gw_addr_prev = 0;
static int dl_ap_gw_icmp_seen = 0;
static int dl_ap_probe_inflight = 0;

static void
dl_concard_write_gw_ok(int ok)
{
\tFILE *fp;
\tif (ok == dl_ap_gw_ok_prev)
\t\treturn;
\tdl_ap_gw_ok_prev = ok;
\tfp = fopen("/tmp/.concard_gw_ok", "w");
\tif (fp) {
\t\tfprintf(fp, "%d\\n", ok ? 1 : 0);
\t\tfclose(fp);
\t}
}

static uint16_t
dl_concard_icmp_csum(const uint8_t *buf, size_t len)
{
\tuint32_t sum = 0;
\tsize_t i;
\tfor (i = 0; i + 1 < len; i += 2)
\t\tsum += ((uint32_t)buf[i] << 8) | buf[i + 1];
\tif (i < len)
\t\tsum += ((uint32_t)buf[i] << 8);
\twhile (sum >> 16)
\t\tsum = (sum & 0xFFFFU) + (sum >> 16);
\treturn (uint16_t)(~sum);
}

static void
dl_concard_drain_icmp_replies(uint32_t expected_gw)
{
\tuint8_t rbuf[160];
\tstruct sockaddr_in from;
\tsocklen_t flen;
\tssize_t n;

\tif (dl_ap_icmp_sock < 0)
\t\treturn;
\tfor (;;) {
\t\tflen = sizeof(from);
\t\tn = recvfrom(dl_ap_icmp_sock, rbuf, sizeof(rbuf), MSG_DONTWAIT,
\t\t             (struct sockaddr *)&from, &flen);
\t\tif (n <= 0)
\t\t\tbreak;
\t\tif (expected_gw != 0 && from.sin_addr.s_addr == expected_gw && n >= 28) {
\t\t\tsize_t ihl = (size_t)(rbuf[0] & 0x0FU) * 4U;
\t\t\tif (ihl >= 20 && (size_t)n >= ihl + 8 &&
\t\t\t    rbuf[ihl] == 0 && rbuf[ihl + 4] == 0xCC && rbuf[ihl + 5] == 0x21) {
\t\t\t\tdl_ap_gw_icmp_seen = 1;
\t\t\t\tdl_ap_probe_inflight = 0;
\t\t\t\tdl_ap_gw_miss_streak = 0;
\t\t\t\tdl_ap_gw_ok = 1;
\t\t\t}
\t\t}
\t}
}

static void
dl_concard_send_gw_icmp(uint32_t gw_saddr)
{
\tuint8_t pkt[16];
\tuint16_t csum;
\tstruct sockaddr_in dst;

\tif (dl_ap_icmp_sock < 0) {
\t\tdl_ap_icmp_sock = socket(AF_INET, SOCK_RAW, IPPROTO_ICMP);
\t\tif (dl_ap_icmp_sock < 0)
\t\t\treturn;
\t}
\tmemset(pkt, 0, sizeof(pkt));
\tpkt[0] = 8; /* ICMP_ECHO */
\tpkt[1] = 0;
\tpkt[4] = 0xCC;
\tpkt[5] = 0x21;
\tpkt[6] = (uint8_t)((dl_ap_icmp_seq >> 8) & 0xFFU);
\tpkt[7] = (uint8_t)(dl_ap_icmp_seq & 0xFFU);
\tdl_ap_icmp_seq++;
\tmemcpy(&pkt[8], "CONCARD!", 8);
\tcsum = htons(dl_concard_icmp_csum(pkt, sizeof(pkt)));
\tmemcpy(&pkt[2], &csum, 2);

\tmemset(&dst, 0, sizeof(dst));
\tdst.sin_family = AF_INET;
\tdst.sin_addr.s_addr = gw_saddr;
\tsendto(dl_ap_icmp_sock, pkt, sizeof(pkt), MSG_DONTWAIT,
\t       (struct sockaddr *)&dst, sizeof(dst));
\tdl_ap_probe_inflight = 1;
}

static int
dl_concard_locate_active(void)
{
\tFILE *fp = fopen("/tmp/.concard_locate_until", "r");
\tunsigned long until_sec = 0, now_sec = 0;
\tif (!fp)
\t\treturn 0;
\tif (fscanf(fp, "%lu", &until_sec) != 1)
\t\tuntil_sec = 0;
\tfclose(fp);
\tif (until_sec == 0)
\t\treturn 0;
\tfp = fopen("/proc/uptime", "r");
\tif (fp) {
\t\tif (fscanf(fp, "%lu", &now_sec) != 1)
\t\t\tnow_sec = 0;
\t\tfclose(fp);
\t}
\treturn (until_sec > now_sec) ? 1 : 0;
}

static unsigned int
dl_concard_read_5port_mask(void)
{
\tunsigned int mask = 0;
\tint p;
\tfor (p = 0; p < 5; p++) {
\t\tunsigned int val = 0;
\t\tif (phy_status_port_speed(p, &val) == 0 && ((val >> 16) & 0x01))
\t\t\tmask |= (1u << p);
\t}
\treturn mask;
}

static int
dl_concard_has_gw_arp(const char *gw_ip)
{
\tFILE *fp;
\tchar line[256];
\tint found = 0;
\tif (!gw_ip || !gw_ip[0] || strcmp(gw_ip, "0.0.0.0") == 0 || strcmp(gw_ip, DEF_LAN_ADDR) == 0)
\t\treturn 1;
\tfp = fopen("/proc/net/arp", "r");
\tif (!fp)
\t\treturn 1;
\tif (fgets(line, sizeof(line), fp)) {
\t\twhile (fgets(line, sizeof(line), fp)) {
\t\t\tchar ip_s[32], hw_s[16], flg_s[16], mac_s[32];
\t\t\tunsigned long flg = 0;
\t\t\tif (sscanf(line, "%31s %15s %15s %31s", ip_s, hw_s, flg_s, mac_s) == 4) {
\t\t\t\tflg = strtoul(flg_s, NULL, 0);
\t\t\t\tif (flg != 0 && strcmp(ip_s, gw_ip) == 0) {
\t\t\t\t\tfound = 1;
\t\t\t\t\tbreak;
\t\t\t\t}
\t\t\t}
\t\t}
\t}
\tfclose(fp);
\treturn found;
}

static void
dl_concard_reacquire_ap_dhcp(const char *reason)
{
\tconst char *hn = get_our_hostname();
\tchar safe_hn[64];
\tsize_t i = 0;
\tif (!hn || !hn[0])
\t\thn = "Concard-N9";
\twhile (*hn && i + 1 < sizeof(safe_hn)) {
\t\tchar c = *hn++;
\t\tif ((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '-' || c == '_')
\t\t\tsafe_hn[i++] = c;
\t\telse if (c == ' ')
\t\t\tsafe_hn[i++] = '-';
\t}
\tsafe_hn[i] = '\\0';
\tif (!safe_hn[0])
\t\tstrcpy(safe_hn, "Concard-N9");

\tdl_ap_last_dhcp_tick = dl_counter_total;
\tlogmessage("detect_link", "AP Smart-DHCP renew (%s, host=%s)", reason, safe_hn);
\tdoSystem("kill -USR2 `cat /var/run/udhcpc_lan.pid 2>/dev/null` 2>/dev/null; "
\t         "kill -TERM `cat /var/run/udhcpc_lan.pid 2>/dev/null` 2>/dev/null; "
\t         "rm -f /var/run/udhcpc_lan.pid; "
\t         "[ -L /tmp/udhcpc_lan.script ] || ln -sf /sbin/rc /tmp/udhcpc_lan.script; "
\t         "/sbin/udhcpc -i %s -s %s -p /var/run/udhcpc_lan.pid -t4 -T2 -d -H %s >/dev/null 2>&1 &",
\t         IFNAME_BR, SCRIPT_UDHCPC_LAN, safe_hn);
}

static void
dl_concard_sync_ap_led(int force)
{
#if defined (BOARD_GPIO_LED_POWER) && defined (BOARD_GPIO_LED_WAN)
\tunsigned int mask;
\tint fault = 0;
\tint led_mode;
\tif (!dl_is_ap_mode || dl_concard_locate_active())
\t\treturn;
\tmask = dl_concard_read_5port_mask();
\tif (mask == 0) {
\t\tfault = 1;
\t} else if (dl_counter_total > 5 && !dl_ap_gw_ok) {
\t\tfault = 1;
\t}
\tled_mode = (nvram_get_int("cc_led_mode") == 1) ? 1 : 0;
\tif (force || fault != dl_ap_led_fault_prev || led_mode != dl_ap_led_mode_prev) {
\t\tdl_ap_led_fault_prev = fault;
\t\tdl_ap_led_mode_prev = led_mode;
\t\tif (fault) {
\t\t\tLED_CONTROL(BOARD_GPIO_LED_POWER, LED_OFF);
\t\t\tLED_CONTROL(BOARD_GPIO_LED_WAN, LED_ON);
\t\t} else if (led_mode == 1) {
\t\t\tLED_CONTROL(BOARD_GPIO_LED_WAN, LED_OFF);
\t\t\tLED_CONTROL(BOARD_GPIO_LED_POWER, LED_OFF);
\t\t} else {
\t\t\tLED_CONTROL(BOARD_GPIO_LED_WAN, LED_OFF);
\t\t\tLED_CONTROL(BOARD_GPIO_LED_POWER, LED_ON);
\t\t}
\t}
#else
\t(void)force;
#endif
}

static void
dl_concard_ap_watchdog_step(void)
{
\tunsigned int mask;
\tint is_dhcp;
\tconst char *ip_cur, *gw_cur;
\tstruct in_addr gw_in;

\tif (!dl_is_ap_mode)
\t\treturn;

\tmask = dl_concard_read_5port_mask();
\tis_dhcp = (nvram_get_int("lan_proto_x") != 0) ? 1 : 0;

\tif (mask == 0) {
\t\tdl_ap_gw_ok = 0;
\t\tdl_ap_gw_miss_streak = 0;
\t\tdl_ap_probe_inflight = 0;
\t\tdl_ap_port_mask_prev = 0;
\t\tdl_concard_write_gw_ok(0);
\t\tdl_concard_sync_ap_led(0);
\t\treturn;
\t}

\tif (is_dhcp && dl_counter_total > 4) {
\t\tunsigned int newly_up = mask & (~dl_ap_port_mask_prev);
\t\tif (newly_up != 0 && (dl_counter_total - dl_ap_last_dhcp_tick) >= 2) {
\t\t\tdl_ap_gw_miss_streak = 0;
\t\t\tdl_ap_probe_inflight = 0;
\t\t\tdl_concard_reacquire_ap_dhcp((dl_ap_port_mask_prev == 0) ? "link-restored" : "port-moved");
\t\t}
\t}

\tip_cur = is_dhcp ? nvram_safe_get("lan_ipaddr_t") : nvram_safe_get("lan_ipaddr");
\tgw_cur = nvram_safe_get("lan_gateway_t");
\tif (!gw_cur || !gw_cur[0] || strcmp(gw_cur, "0.0.0.0") == 0)
\t\tgw_cur = nvram_safe_get("lan_gateway");

\tif (is_dhcp && (!ip_cur || !ip_cur[0] || strcmp(ip_cur, "0.0.0.0") == 0 || strcmp(ip_cur, DEF_LAN_ADDR) == 0)) {
\t\tdl_ap_gw_ok = 0;
\t\tdl_ap_probe_inflight = 0;
\t\tif (dl_counter_total > 5 && (dl_counter_total - dl_ap_last_dhcp_tick) >= 6)
\t\t\tdl_concard_reacquire_ap_dhcp("fallback-ip-retry");
\t} else if (gw_cur && gw_cur[0] && strcmp(gw_cur, "0.0.0.0") != 0 &&
\t           strcmp(gw_cur, DEF_LAN_ADDR) != 0 && inet_aton(gw_cur, &gw_in) != 0) {
\t\tif (gw_in.s_addr != dl_ap_gw_addr_prev) {
\t\t\tdl_ap_gw_addr_prev = gw_in.s_addr;
\t\t\tdl_ap_gw_icmp_seen = 0;
\t\t\tdl_ap_gw_miss_streak = 0;
\t\t\tdl_ap_probe_inflight = 0;
\t\t}

\t\tdl_concard_drain_icmp_replies(gw_in.s_addr);

\t\tif (dl_ap_probe_inflight) {
\t\t\tif (dl_ap_gw_icmp_seen) {
\t\t\t\tdl_ap_gw_miss_streak++;
\t\t\t} else if (dl_concard_has_gw_arp(gw_cur)) {
\t\t\t\tdl_ap_gw_miss_streak = 0;
\t\t\t\tdl_ap_gw_ok = 1;
\t\t\t} else {
\t\t\t\tdl_ap_gw_miss_streak++;
\t\t\t}

\t\t\tif (dl_ap_gw_miss_streak >= 3) {
\t\t\t\tdl_ap_gw_ok = 0;
\t\t\t\tif (is_dhcp && (dl_counter_total - dl_ap_last_dhcp_tick) >= 8) {
\t\t\t\t\tdl_ap_gw_miss_streak = 0;
\t\t\t\t\tdl_concard_reacquire_ap_dhcp("gateway-unreachable");
\t\t\t\t}
\t\t\t}
\t\t}

\t\t/* Send 16-byte non-blocking ICMP Echo every 2s tick (~6s for 3 missed checks) */
\t\tdl_concard_send_gw_icmp(gw_in.s_addr);
\t} else {
\t\tdl_ap_gw_ok = 1;
\t}

\tdl_ap_port_mask_prev = mask;
\tdl_concard_write_gw_ok(dl_ap_gw_ok);
\tdl_concard_sync_ap_led(0);
}

"""

    if "/* CONCARD_V2_AP_WATCHDOG_LED:" in dl:
        start_v2 = dl.index("/* CONCARD_V2_AP_WATCHDOG_LED:")
        end_v2 = dl.index(anchor_reset, start_v2)
        dl = dl[:start_v2] + v21_helpers + dl[end_v2:]
    else:
        dl = dl.replace(anchor_reset, v21_helpers + anchor_reset, 1)
        old_timer_end = "\tdl_handle_link_lan();\n#if defined (BOARD_GPIO_LED_USB) && defined (USE_USB_SUPPORT)\n\tdl_handle_link_usb(0);\n#endif\n}"
        new_timer_end = (
            "\tdl_handle_link_lan();\n"
            "#if defined (BOARD_GPIO_LED_USB) && defined (USE_USB_SUPPORT)\n"
            "\tdl_handle_link_usb(0);\n"
            "#endif\n"
            "\tdl_concard_ap_watchdog_step();\n"
            "}"
        )
        assert old_timer_end in dl, "detect_link.c: dl_on_timer end anchor not found"
        dl = dl.replace(old_timer_end, new_timer_end, 1)

        old_upd_end = "#if defined (BOARD_GPIO_LED_POWER)\n\tLED_CONTROL(BOARD_GPIO_LED_POWER, LED_ON);\n#endif"
        new_upd_end = (
            "#if defined (BOARD_GPIO_LED_POWER)\n"
            "\tif (!dl_is_ap_mode)\n"
            "\t\tLED_CONTROL(BOARD_GPIO_LED_POWER, LED_ON);\n"
            "#endif\n"
            "\tif (dl_is_ap_mode)\n"
            "\t\tdl_concard_sync_ap_led(1);"
        )
        assert old_upd_end in dl, "detect_link.c: dl_update_leds BOARD_GPIO_LED_POWER anchor not found"
        dl = dl.replace(old_upd_end, new_upd_end, 1)

    with open(dl_path, "w", encoding="utf-8", newline="\n") as f:
        f.write(dl)
    dl_o = os.path.join(TRUNK, "user/rc/detect_link.o")
    if os.path.exists(dl_o):
        os.remove(dl_o)
    print("[OK] Patched user/rc/detect_link.c (CONCARD_V21_AP_WATCHDOG_LED: L3 ICMP+ARP Static/DHCP + Night Mode)")


def patch_webui():
    var_path = os.path.join(TRUNK, "user/httpd/variables.c")
    with open(var_path, "r", encoding="utf-8", errors="replace") as f:
        vtxt = f.read()

    v_changed = False
    if "rt_ft_enable" not in vtxt:
        old_v = (
            '\t\t\t{"rt_KickStaRssiLow", "", NULL, EVM_RESTART_WIFI2},\n'
            '\t\t\t{"rt_AssocReqRssiThres", "", NULL, EVM_RESTART_WIFI2},'
        )
        new_v = (
            '\t\t\t{"rt_KickStaRssiLow", "", NULL, EVM_RESTART_WIFI2},\n'
            '\t\t\t{"rt_AssocReqRssiThres", "", NULL, EVM_RESTART_WIFI2},\n'
            '\t\t\t{"rt_rrm_enable", "", NULL, EVM_RESTART_WIFI2},\n'
            '\t\t\t{"rt_ft_enable", "", NULL, EVM_RESTART_WIFI2},\n'
            '\t\t\t{"rt_ft_mdid", "", NULL, EVM_RESTART_WIFI2},\n'
            '\t\t\t{"rt_ft_r0kh_id", "", NULL, EVM_RESTART_WIFI2},\n'
            '\t\t\t{"cc_guest_vlan", "", NULL, EVM_RESTART_WIFI2},'
        )
        assert old_v in vtxt, "variables.c: target not found"
        vtxt = vtxt.replace(old_v, new_v, 1)
        v_changed = True

    if '"cc_qload_enable"' not in vtxt:
        anchor_gv = '\t\t\t{"cc_guest_vlan", "", NULL, EVM_RESTART_WIFI2},'
        assert anchor_gv in vtxt, "variables.c: cc_guest_vlan anchor not found"
        vtxt = vtxt.replace(
            anchor_gv,
            anchor_gv
            + '\n\t\t\t{"cc_qload_enable", "", NULL, EVM_RESTART_WIFI2},'
            + '\n\t\t\t{"cc_probe_rssi", "", NULL, EVM_RESTART_WIFI2},',
            1,
        )
        v_changed = True

    if '"cc_device_loc"' not in vtxt:
        anchor_cn = '\t\t\t{"computer_name", "", NULL, EVM_RESTART_DHCPD|EVM_RESTART_FTPD|EVM_RESTART_NMBD|EVM_RESTART_DMS|EVM_RESTART_ITUNES},'
        assert anchor_cn in vtxt, "variables.c: computer_name anchor not found"
        vtxt = vtxt.replace(
            anchor_cn,
            anchor_cn
            + '\n\t\t\t{"cc_device_loc", "", NULL, FALSE},'
            + '\n\t\t\t{"cc_cell_breathing", "", NULL, FALSE},'
            + '\n\t\t\t{"cc_ctrl_url", "", NULL, FALSE},',
            1,
        )
        v_changed = True

    if '"cc_ota_repo_url"' not in vtxt:
        anchor_cu = '\t\t\t{"cc_ctrl_url", "", NULL, FALSE},'
        assert anchor_cu in vtxt, "variables.c: cc_ctrl_url anchor not found"
        vtxt = vtxt.replace(
            anchor_cu,
            anchor_cu
            + '\n\t\t\t{"cc_fair_queue", "", NULL, FALSE},'
            + '\n\t\t\t{"cc_l2_shield", "", NULL, FALSE},'
            + '\n\t\t\t{"cc_led_mode", "", NULL, FALSE},'
            + '\n\t\t\t{"cc_ota_repo_url", "", NULL, FALSE},',
            1,
        )
        v_changed = True

    if v_changed:
        with open(var_path, "w", encoding="utf-8", newline="\n") as f:
            f.write(vtxt)
        var_o = os.path.join(TRUNK, "user/httpd/variables.o")
        if os.path.exists(var_o):
            os.remove(var_o)
        print("[OK] Patched user/httpd/variables.c (added v2.1 wireless, fairness, L2 shield, LED & OTA variables)")

    nw_path = os.path.join(TRUNK, "user/rc/net_wifi.c")
    with open(nw_path, "r", encoding="utf-8", errors="replace") as f:
        nwtxt = f.read()
    if "concard-vlan-apply" not in nwtxt:
        old_nw = (
            "\t\t\twif_control_m2u(0, IFNAME_2G_GUEST);\n"
            "\t\t\tset_wifi_rssi_threshold(IFNAME_2G_GUEST, 0);\n"
            "\t\t}\n"
            "\t}"
        )
        new_nw = (
            "\t\t\twif_control_m2u(0, IFNAME_2G_GUEST);\n"
            "\t\t\tset_wifi_rssi_threshold(IFNAME_2G_GUEST, 0);\n"
            "\t\t}\n"
            '\t\tdoSystem("/usr/sbin/concard-vlan-apply >/dev/null 2>&1 &");\n'
            "\t}"
        )
        assert old_nw in nwtxt, "net_wifi.c: start_wifi_ap_rt guest block not found"
        nwtxt = nwtxt.replace(old_nw, new_nw, 1)
        with open(nw_path, "w", encoding="utf-8", newline="\n") as f:
            f.write(nwtxt)
        print("[OK] Patched user/rc/net_wifi.c (auto-invoke concard-vlan-apply)")


def patch_mtd_storage():
    sh_path = os.path.join(TRUNK, "user/scripts/mtd_storage.sh")
    with open(sh_path, "r", encoding="utf-8", errors="replace") as f:
        stxt = f.read()

    changed = False
    if "ConCard Enterprise AP Tuning" not in stxt:
        old_s = """#drop caches
sync && echo 3 > /proc/sys/vm/drop_caches"""

        new_s = """# ConCard Enterprise AP Tuning: High Conntrack + FQ-CoDel Anti-Bufferbloat
sysctl -w net.netfilter.nf_conntrack_max=32768 >/dev/null 2>&1
tc qdisc replace dev eth2 root fq_codel quantum 300 limit 1024 noecn >/dev/null 2>&1
tc qdisc replace dev ra0 root fq_codel quantum 300 limit 1024 noecn >/dev/null 2>&1

#drop caches
sync && echo 3 > /proc/sys/vm/drop_caches"""

        assert old_s in stxt, "mtd_storage.sh: target not found"
        stxt = stxt.replace(old_s, new_s, 1)
        changed = True

    if "concard-watchdog" not in stxt:
        old_drop = "#drop caches\nsync && echo 3 > /proc/sys/vm/drop_caches"
        new_drop = (
            "[ -x /usr/sbin/concard-vlan-apply ] && /usr/sbin/concard-vlan-apply >/dev/null 2>&1 &\n"
            "[ -x /usr/sbin/concard-watchdog ] && /usr/sbin/concard-watchdog >/dev/null 2>&1 &\n\n"
            + old_drop
        )
        assert old_drop in stxt, "mtd_storage.sh: #drop caches anchor not found"
        stxt = stxt.replace(old_drop, new_drop, 1)
        changed = True

    if "concard-dhcp-fp" not in stxt:
        old_wd = "[ -x /usr/sbin/concard-watchdog ] && /usr/sbin/concard-watchdog >/dev/null 2>&1 &"
        new_wd = (
            "[ -x /usr/sbin/concard-dhcp-fp ] && /usr/sbin/concard-dhcp-fp >/dev/null 2>&1 &\n"
            + old_wd
        )
        assert old_wd in stxt, "mtd_storage.sh: concard-watchdog anchor not found"
        stxt = stxt.replace(old_wd, new_wd, 1)
        changed = True

    if changed:
        with open(sh_path, "w", encoding="utf-8", newline="\n") as f:
            f.write(stxt)
        print("[OK] Patched user/scripts/mtd_storage.sh (FQ-CoDel + L2 Shield + DHCP-FP + Watchdog)")
    else:
        print("[OK] mtd_storage.sh already patched for Concard v2.1")


def patch_dropbear_harden():
    db_path = os.path.join(TRUNK, "user/dropbear/Makefile")
    with open(db_path, "r", encoding="utf-8", errors="replace") as f:
        dtxt = f.read()
    if "--disable-harden" not in dtxt:
        dtxt = dtxt.replace("--enable-zlib \\", "--disable-harden \\\n\t\t--enable-zlib \\", 1)
        with open(db_path, "w", encoding="utf-8") as f:
            f.write(dtxt)
        print("[OK] Patched user/dropbear/Makefile (--disable-harden for uClibc)")
    else:
        print("[OK] user/dropbear/Makefile already patched")


def patch_security_signature_lock():
    """Item 6: Lock /upgrade.cgi strictly to BOARD_PID (CSB334A-2700) & disable UDP 9999 infosvr."""
    up_path = os.path.join(TRUNK, "user/httpd/upload.c")
    with open(up_path, "r", encoding="utf-8", errors="replace") as f:
        utxt = f.read()
    up_changed = False

    old_cross = '\tif (strcmp(pid_asus, BOARD_PID) != 0 && strncmp(buf + 32, "CSB334A-", 8) != 0) {'
    strict_lock = "\tif (strcmp(pid_asus, BOARD_PID) != 0) { /* CONCARD_V21_SIG_LOCK */"
    if old_cross in utxt:
        utxt = utxt.replace(old_cross, strict_lock, 1)
        up_changed = True
    elif "CONCARD_V21_SIG_LOCK" not in utxt:
        orig_lock = "\tif (strcmp(pid_asus, BOARD_PID) != 0) {"
        assert orig_lock in utxt, "upload.c: BOARD_PID check not found"
        utxt = utxt.replace(orig_lock, strict_lock, 1)
        up_changed = True

    old_9999 = (
        '\tif (strncmp(buf + 32, "CSB334A-9999", 12) != 0)\n'
        "\t\t*file_len = (long)(ntohl(hdr->ih_size) + sizeof(image_header_t));"
    )
    strict_len = "\t*file_len = (long)(ntohl(hdr->ih_size) + sizeof(image_header_t));"
    if old_9999 in utxt:
        utxt = utxt.replace(old_9999, strict_len, 1)
        up_changed = True

    if up_changed:
        with open(up_path, "w", encoding="utf-8", newline="\n") as f:
            f.write(utxt)
        up_o = os.path.join(TRUNK, "user/httpd/upload.o")
        if os.path.exists(up_o):
            os.remove(up_o)
        print("[OK] Locked user/httpd/upload.c signature check strictly to CSB334A-2700 (anti-rollback)")
    else:
        print("[OK] user/httpd/upload.c already locked to CSB334A-2700")

    svc_path = os.path.join(TRUNK, "user/rc/services.c")
    with open(svc_path, "r", encoding="utf-8", errors="replace") as f:
        stxt = f.read()
    if "CONCARD_DISABLE_INFOSVR" not in stxt:
        old_info = "start_infosvr(void)\n{\n\tif (nvram_invmatch(\"adsc_enable\", \"1\"))"
        new_info = (
            "start_infosvr(void)\n{\n"
            "#ifdef BOARD_N9V1\n"
            "\t/* CONCARD_DISABLE_INFOSVR: Never expose UDP 9999 L2 discovery on hotel APs */\n"
            "\treturn 0;\n"
            "#endif\n"
            "\tif (nvram_invmatch(\"adsc_enable\", \"1\"))"
        )
        assert old_info in stxt, "services.c: start_infosvr anchor not found"
        stxt = stxt.replace(old_info, new_info, 1)
        with open(svc_path, "w", encoding="utf-8", newline="\n") as f:
            f.write(stxt)
        svc_o = os.path.join(TRUNK, "user/rc/services.o")
        if os.path.exists(svc_o):
            os.remove(svc_o)
        print("[OK] Disabled infosvr (UDP 9999) in user/rc/services.c")
    else:
        print("[OK] infosvr already disabled in user/rc/services.c")


def patch_rust_wasm_webui():
    # 0. Set Concard N9 v2.1 firmware revision tag in versions.inc
    ver_inc_path = os.path.join(TRUNK, "versions.inc")
    if os.path.isfile(ver_inc_path):
        with open(ver_inc_path, "r", encoding="utf-8", errors="replace") as f:
            vinc = f.read()
        new_vinc_lines = []
        vinc_changed = False
        for line in vinc.splitlines():
            if line.startswith("FIRMWARE_BUILDS_REV="):
                if line != "FIRMWARE_BUILDS_REV=Concard-N9-v2.1":
                    vinc_changed = True
                new_vinc_lines.append("FIRMWARE_BUILDS_REV=Concard-N9-v2.1")
            else:
                new_vinc_lines.append(line)
        if vinc_changed:
            with open(ver_inc_path, "w", encoding="utf-8", newline="\n") as f:
                f.write("\n".join(new_vinc_lines) + "\n")
            common_ex_o = os.path.join(TRUNK, "user/rc/common_ex.o")
            if os.path.exists(common_ex_o):
                os.remove(common_ex_o)
            print("[OK] Updated versions.inc -> FIRMWARE_BUILDS_REV=Concard-N9-v2.1")

    # 1. Install ConCard Multi-User Auth + ConCard v2.1 C Telemetry/Locate/OTA CGI engines into user/httpd
    httpd_dir = os.path.join(TRUNK, "user/httpd")
    for fname in ("concard_auth.h", "concard_auth.c", "concard_v2.h", "concard_v2.c"):
        shutil.copyfile(os.path.join(WIN_BSP_DIR, fname), os.path.join(httpd_dir, fname))
    for stale_o in ("concard_auth.o", "concard_v2.o"):
        so_path = os.path.join(httpd_dir, stale_o)
        if os.path.exists(so_path):
            os.remove(so_path)

    httpd_mk_path = os.path.join(httpd_dir, "Makefile")
    with open(httpd_mk_path, "r", encoding="utf-8", errors="replace") as f:
        hmk = f.read()
    hmk_changed = False
    if "concard_auth.o" not in hmk:
        anchor_mk = "OBJS += tdate_parse.o upload.o"
        assert anchor_mk in hmk, "user/httpd/Makefile: OBJS anchor not found"
        hmk = hmk.replace(anchor_mk, anchor_mk + "\nOBJS += concard_auth.o", 1)
        hmk_changed = True
    if "concard_v2.o" not in hmk:
        anchor_mk2 = "OBJS += concard_auth.o"
        assert anchor_mk2 in hmk, "user/httpd/Makefile: concard_auth.o anchor not found"
        hmk = hmk.replace(anchor_mk2, anchor_mk2 + " concard_v2.o", 1)
        hmk_changed = True
    if hmk_changed:
        with open(httpd_mk_path, "w", encoding="utf-8", newline="\n") as f:
            f.write(hmk)
        print("[OK] Patched user/httpd/Makefile (added concard_auth.o + concard_v2.o)")

    # 2. Patch user/httpd/web_ex.c MIME handlers + instant concard-vlan-apply hook in ej_notify_services
    web_ex_path = os.path.join(httpd_dir, "web_ex.c")
    with open(web_ex_path, "r", encoding="utf-8", errors="replace") as f:
        wex = f.read()

    wex_changed = False
    if '#include "concard_auth.h"' not in wex:
        assert '#include "httpd.h"' in wex, "web_ex.c: #include httpd.h not found"
        wex = wex.replace('#include "httpd.h"', '#include "httpd.h"\n#include "concard_auth.h"', 1)
        wex_changed = True
    if '#include "concard_v2.h"' not in wex:
        assert '#include "concard_auth.h"' in wex, "web_ex.c: #include concard_auth.h not found"
        wex = wex.replace('#include "concard_auth.h"', '#include "concard_auth.h"\n#include "concard_v2.h"', 1)
        wex_changed = True

    anchor_md5 = '\t{ "md5.js", "text/javascript", NULL, NULL, do_file, 0 }, // 2012.06 Eagle23'
    if '"api_users.cgi*"' not in wex:
        new_wasm_v12 = (
            anchor_md5
            + '\n\t{ "index.asp", "text/html", no_cache_IE, NULL, do_file, 0 },'
            + '\n\t{ "**ap_ui.js", "text/javascript", NULL, NULL, do_file, 0 },'
            + '\n\t{ "**.wasm", "application/wasm", NULL, NULL, do_file, 0 },'
            + '\n\t{ "api_auth.cgi*", "application/json", no_cache_IE, do_concard_auth_post, do_concard_auth_cgi, 0 },'
            + '\n\t{ "api_users.cgi*", "application/json", no_cache_IE, do_concard_users_post, do_concard_users_cgi, 1 },'
        )
        assert anchor_md5 in wex, "web_ex.c: md5.js anchor not found"
        wex = wex.replace(anchor_md5, new_wasm_v12, 1)
        wex_changed = True

    if '"api_telemetry.cgi*"' not in wex:
        anchor_u_cgi = '\t{ "api_users.cgi*", "application/json", no_cache_IE, do_concard_users_post, do_concard_users_cgi, 1 },'
        assert anchor_u_cgi in wex, "web_ex.c: api_users.cgi* anchor not found"
        wex = wex.replace(
            anchor_u_cgi,
            anchor_u_cgi
            + '\n\t{ "api_telemetry.cgi*", "application/json", no_cache_IE, NULL, do_concard_telemetry_cgi, 1 },'
            + '\n\t{ "api_locate.cgi*", "application/json", no_cache_IE, do_concard_locate_post, do_concard_locate_cgi, 1 },',
            1,
        )
        wex_changed = True

    if '"api_ota.cgi*"' not in wex:
        anchor_loc_cgi = '\t{ "api_locate.cgi*", "application/json", no_cache_IE, do_concard_locate_post, do_concard_locate_cgi, 1 },'
        assert anchor_loc_cgi in wex, "web_ex.c: api_locate.cgi* anchor not found"
        wex = wex.replace(
            anchor_loc_cgi,
            anchor_loc_cgi
            + '\n\t{ "api_ota.cgi*", "application/json", no_cache_IE, do_concard_ota_post, do_concard_ota_cgi, 1 },',
            1,
        )
        wex_changed = True

    if "CONCARD_NOTIFY_VLAN_APPLY" not in wex:
        old_ns = "ej_notify_services(int eid, webs_t wp, int argc, char **argv)\n{\n\tint i;\n\n\tif (!restart_needed_bits)\n\t\treturn 0;"
        new_ns = (
            "ej_notify_services(int eid, webs_t wp, int argc, char **argv)\n{\n\tint i;\n\n"
            "\t/* CONCARD_NOTIFY_VLAN_APPLY: Apply LED mode, FQ-CoDel, L2 shield & ProbeRspRssi immediately */\n"
            "\tdoSystem(\"/usr/sbin/concard-vlan-apply >/dev/null 2>&1 &\");\n\n"
            "\tif (!restart_needed_bits)\n\t\treturn 0;"
        )
        assert old_ns in wex, "web_ex.c: ej_notify_services anchor not found"
        wex = wex.replace(old_ns, new_ns, 1)
        wex_changed = True

    if wex_changed:
        with open(web_ex_path, "w", encoding="utf-8", newline="\n") as f:
            f.write(wex)
        wex_o = os.path.join(httpd_dir, "web_ex.o")
        if os.path.exists(wex_o):
            os.remove(wex_o)
        print("[OK] Patched user/httpd/web_ex.c (api_ota.cgi + instant concard-vlan-apply on Save)")

    # 3. Patch user/httpd/httpd.c (Cookie sessions + Basic Auth + Viewer RBAC + Controller header hook)
    httpd_c_path = os.path.join(httpd_dir, "httpd.c")
    with open(httpd_c_path, "r", encoding="utf-8", errors="replace") as f:
        hc = f.read()

    if "CONCARD_MULTIUSER_SESSION_AUTH" not in hc:
        assert '#include "httpd.h"' in hc, "httpd.c: #include httpd.h not found"
        hc = hc.replace(
            '#include "httpd.h"',
            '#include "httpd.h"\n#include "concard_auth.h" /* CONCARD_MULTIUSER_SESSION_AUTH */',
            1,
        )
        old_sync = "\tif (auth_nvram_changed) {\n\t\tauth_nvram_changed = 0;\n\t\tload_nvram_auth();\n\t}"
        new_sync = (
            "\tif (auth_nvram_changed) {\n"
            "\t\tauth_nvram_changed = 0;\n"
            "\t\tload_nvram_auth();\n"
            "\t\tconcard_auth_sync_nvram();\n"
            "\t}"
        )
        assert old_sync in hc, "httpd.c: auth_nvram_changed block not found"
        hc = hc.replace(old_sync, new_sync, 1)

        old_shdr = (
            '\tfprintf( conn_fp, "Server: %s\\r\\n", SERVER_NAME );\n'
            '\tfprintf( conn_fp, "Date: %s\\r\\n", timebuf );\n'
            "\tif (extra_header) {\n"
            '\t\tfprintf( conn_fp, "%s\\r\\n", extra_header );\n'
        )
        new_shdr = (
            '\tfprintf( conn_fp, "Server: %s\\r\\n", SERVER_NAME );\n'
            '\tfprintf( conn_fp, "X-ConCard-AP: Padavan\\r\\n" );\n'
            '\tfprintf( conn_fp, "Date: %s\\r\\n", timebuf );\n'
            "\tif (concard_auth_get_extra_header()) {\n"
            '\t\tfprintf( conn_fp, "%s\\r\\n", concard_auth_get_extra_header() );\n'
            "\t} else if (extra_header) {\n"
            '\t\tfprintf( conn_fp, "%s\\r\\n", extra_header );\n'
        )
        assert old_shdr in hc, "httpd.c: send_headers block not found"
        hc = hc.replace(old_shdr, new_shdr, 1)

        old_vars = "\t/* Initialize the request variables. */\n\tauthorization = boundary = NULL;"
        new_vars = (
            "\tchar cookie_buf[512] = {0};\n"
            "\tchar cc_user[33] = {0}, cc_role[12] = {0};\n"
            "\tint has_browser_hdr = 0, cc_authed = 0;\n"
            "\t/* Initialize the request variables. */\n"
            "\tauthorization = boundary = NULL;\n"
            "\tconcard_auth_begin_request();"
        )
        assert old_vars in hc, "httpd.c: request variable init not found"
        hc = hc.replace(old_vars, new_vars, 1)

        old_hdr_loop = (
            '\t\telse if (strncasecmp( cur, "If-Modified-Since:", 18) == 0) {\n'
            "\t\t\tcp = cur + 18;\n"
            '\t\t\tcp += strspn( cp, " \\t" );\n'
            "\t\t\tif_modified_since = tdate_parse(cp);\n"
            "\t\t}"
        )
        new_hdr_loop = (
            old_hdr_loop
            + '\n\t\telse if (strncasecmp( cur, "Cookie:", 7) == 0) {\n'
            "\t\t\tchar *ce;\n"
            "\t\t\tcp = cur + 7;\n"
            '\t\t\tcp += strspn( cp, " \\t" );\n'
            '\t\t\tsnprintf(cookie_buf, sizeof(cookie_buf), "%s", cp);\n'
            '\t\t\tce = strpbrk(cookie_buf, "\\r\\n");\n'
            "\t\t\tif (ce) *ce = '\\0';\n"
            "\t\t}\n"
            '\t\telse if (strncasecmp( cur, "Accept:", 7) == 0 || strncasecmp( cur, "User-Agent:", 11) == 0) {\n'
            "\t\t\thas_browser_hdr = 1;\n"
            "\t\t}"
        )
        assert old_hdr_loop in hc, "httpd.c: If-Modified-Since header loop not found"
        hc = hc.replace(old_hdr_loop, new_hdr_loop, 1)

        old_login_blk = (
            "\tlogin_state = http_login_check(&conn_ip);\n"
            "\t\n"
            "\tif (login_state == 0) {\n"
            '\t\tif (strstr(file, ".htm") != NULL || strstr(file, ".asp") != NULL) {\n'
            '\t\t\tfile = "Nologin.asp";\n'
            "\t\t\tquery = NULL;\n"
            "\t\t}\n"
            "\t}"
        )
        new_login_blk = (
            "\tlogin_state = is_uaddr_localhost(&conn_ip) ? 1 : 2;\n"
            "\tcc_authed = concard_auth_check_request(cookie_buf, authorization, cc_user, sizeof(cc_user), cc_role, sizeof(cc_role));\n"
            "\tif (cc_authed)\n"
            "\t\thttp_login(&conn_ip);\n"
            '\tif (strcmp(file, "index.asp") == 0 && !cc_authed && !has_browser_hdr && !cookie_buf[0] && !authorization) {\n'
            "\t\tsend_authenticate(conn_fp);\n"
            "\t\treturn;\n"
            "\t}"
        )
        assert old_login_blk in hc, "httpd.c: login_state Nologin.asp block not found"
        hc = hc.replace(old_login_blk, new_login_blk, 1)

        old_auth_gate = (
            "\tif (handler->need_auth && login_state > 1 && !do_logout) {\n"
            "\t\tif (!auth_check(authorization)) {\n"
            "\t\t\thttp_logout(&conn_ip);\n"
            "\t\t\tif (method_id == HTTP_METHOD_POST)\n"
            "\t\t\t\teat_post_data(conn_fp, clen);\n"
            "\t\t\tsend_authenticate(conn_fp);\n"
            "\t\t\treturn;\n"
            "\t\t}\n"
            "\t\t\n"
            "\t\tif (login_state == 2)\n"
            "\t\t\thttp_login(&conn_ip);\n"
            "\t}"
        )
        new_auth_gate = (
            "\tif (handler->need_auth && login_state != 1 && !do_logout) {\n"
            "\t\tif (!cc_authed) {\n"
            "\t\t\tif (method_id == HTTP_METHOD_POST)\n"
            "\t\t\t\teat_post_data(conn_fp, clen);\n"
            '\t\t\tsend_headers(401, "Unauthorized", "Cache-Control: no-store, no-cache", "application/json", NULL, conn_fp);\n'
            '\t\t\tfprintf(conn_fp, "{\\"ok\\":false,\\"authenticated\\":false,\\"auth_required\\":true}\\n");\n'
            "\t\t\tfflush(conn_fp);\n"
            "\t\t\treturn;\n"
            "\t\t}\n"
            "\t\tif (strcmp(cc_role, CONCARD_ROLE_VIEWER) == 0 && concard_auth_is_viewer_forbidden(file, method_id)) {\n"
            "\t\t\tif (method_id == HTTP_METHOD_POST)\n"
            "\t\t\t\teat_post_data(conn_fp, clen);\n"
            '\t\t\tsend_headers(403, "Forbidden", "Cache-Control: no-store, no-cache", "application/json", NULL, conn_fp);\n'
            '\t\t\tfprintf(conn_fp, "{\\"ok\\":false,\\"error\\":\\"Read-only Viewer account: Administrator privileges required.\\"}\\n");\n'
            "\t\t\tfflush(conn_fp);\n"
            "\t\t\treturn;\n"
            "\t\t}\n"
            "\t}"
        )
        assert old_auth_gate in hc, "httpd.c: handler->need_auth gate not found"
        hc = hc.replace(old_auth_gate, new_auth_gate, 1)

        old_main_init = "\treset_login_data();\n\tload_nvram_auth();"
        new_main_init = "\treset_login_data();\n\tload_nvram_auth();\n\tconcard_auth_init();"
        assert old_main_init in hc, "httpd.c: main load_nvram_auth not found"
        hc = hc.replace(old_main_init, new_main_init, 1)

        with open(httpd_c_path, "w", encoding="utf-8", newline="\n") as f:
            f.write(hc)

    if "concard_v2_note_controller" not in hc:
        if '#include "concard_v2.h"' not in hc:
            hc = hc.replace('#include "concard_auth.h"', '#include "concard_auth.h"\n#include "concard_v2.h"', 1)
        anchor_b_hdr = (
            '\t\telse if (strncasecmp( cur, "Accept:", 7) == 0 || strncasecmp( cur, "User-Agent:", 11) == 0) {\n'
            "\t\t\thas_browser_hdr = 1;\n"
            "\t\t}"
        )
        new_ctrl_hdr = (
            anchor_b_hdr
            + '\n\t\telse if (strncasecmp( cur, "X-ConCard-Controller:", 21) == 0) {\n'
            "\t\t\tcp = cur + 21;\n"
            '\t\t\tcp += strspn( cp, " \\t" );\n'
            "\t\t\tconcard_v2_note_controller(cp);\n"
            "\t\t}"
        )
        assert anchor_b_hdr in hc, "httpd.c: Accept/User-Agent header anchor not found"
        hc = hc.replace(anchor_b_hdr, new_ctrl_hdr, 1)
        with open(httpd_c_path, "w", encoding="utf-8", newline="\n") as f:
            f.write(hc)
        httpd_o = os.path.join(httpd_dir, "httpd.o")
        if os.path.exists(httpd_o):
            os.remove(httpd_o)

    # 4. Build (if cargo present) & stage Rust -> WebAssembly UI
    webui_dir = os.path.join(WIN_CUSTOM_DIR, "webui")
    cargo_toml = os.path.join(webui_dir, "Cargo.toml")
    dist_wasm = os.path.join(webui_dir, "dist", "ap_ui.wasm")
    os.makedirs(os.path.join(webui_dir, "dist"), exist_ok=True)

    cargo_bin = shutil.which("cargo")
    if not cargo_bin:
        for cand in ("/root/.cargo/bin/cargo", os.path.expanduser("~/.cargo/bin/cargo")):
            if os.path.isfile(cand) and os.access(cand, os.X_OK):
                cargo_bin = cand
                break

    if cargo_bin and os.path.isfile(cargo_toml):
        print(f"[INFO] Building Rust -> WebAssembly Admin UI v2.1 via {cargo_bin}...")
        env = os.environ.copy()
        cargo_dir = os.path.dirname(cargo_bin)
        env["PATH"] = f"{cargo_dir}:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:{env.get('PATH', '')}"
        env["CARGO_TARGET_DIR"] = "/tmp/n9v1_wasm_build"
        subprocess.run(
            [
                cargo_bin,
                "build",
                "--manifest-path",
                cargo_toml,
                "--target",
                "wasm32-unknown-unknown",
                "--release",
            ],
            env=env,
            check=True,
        )
        built_wasm = "/tmp/n9v1_wasm_build/wasm32-unknown-unknown/release/n9v1_ap_webui.wasm"
        shutil.copyfile(built_wasm, dist_wasm)
        print(f"[OK] Compiled {dist_wasm} ({os.path.getsize(dist_wasm)} bytes)")
    else:
        assert os.path.isfile(dist_wasm), f"Prebuilt WASM bundle missing at {dist_wasm}"
        print(f"[OK] Using prebuilt {dist_wasm} ({os.path.getsize(dist_wasm)} bytes)")

    # 5. Build lean Classic-UI-free web root in user/www/concard_minimal
    orig_www = os.path.join(TRUNK, "user/www/n56u_ribbon_fixed")
    classic_asp = os.path.join(orig_www, "classic.asp")
    if os.path.exists(classic_asp):
        os.remove(classic_asp)

    min_www = os.path.join(TRUNK, "user/www/concard_minimal")
    if os.path.exists(min_www):
        shutil.rmtree(min_www)
    os.makedirs(os.path.join(min_www, "images"), exist_ok=True)
    os.makedirs(os.path.join(min_www, "device-map"), exist_ok=True)

    shutil.copyfile(dist_wasm, os.path.join(min_www, "ap_ui.wasm"))
    for fname in ("ap_ui.js", "ap_ui.css", "ap_state.asp", "index.asp"):
        shutil.copyfile(os.path.join(webui_dir, "static", fname), os.path.join(min_www, fname))

    fav_src = os.path.join(orig_www, "images", "favicon.png")
    if os.path.isfile(fav_src):
        shutil.copyfile(fav_src, os.path.join(min_www, "images", "favicon.png"))

    headless_stubs = {
        "httpd_check.htm": "ASUSTeK\n",
        "EN.dict": "LANG_EN=English\nWeb_Title=Concard N9 v2.1\n",
        "start_apply.htm": (
            "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><script>\n"
            "var restart_time = 0;\n"
            "function done_validating(a){}\n"
            "function done_committing(){}\n"
            "function no_changes_and_no_committing(){}\n"
            "function restart_needed_time(s){ restart_time = s; }\n"
            "<% get_flash_time(); %>\n"
            "</script></head><body>\n"
            "<% update_variables(); %>\n"
            "<% asus_nvram_commit(); %>\n"
            "<% notify_services(); %>\n"
            "OK\n</body></html>\n"
        ),
        "Main_WStatus2g_Content.asp": "<textarea readonly=\"readonly\"><% nvram_dump(\"wlan11b_2g.log\",\"\"); %></textarea>\n",
        "device-map/clients.asp": "var ipmonitor = [<% get_static_client(); %>];\n",
        "wds_aplist_2g.asp": "wds_aplist = <% wl_scan_2g(); %>;\n",
        "Advanced_Wireless2g_Content.asp": (
            "<input type=\"hidden\" name=\"rt_ssid2\" value=\"<% nvram_char_to_ascii(\"\", \"rt_ssid\"); %>\">\n"
            "<input type=\"hidden\" name=\"rt_channel_orig\" value=\"<% nvram_get_x(\"\", \"rt_channel\"); %>\">\n"
            "<input type=\"hidden\" name=\"rt_TxPower\" value=\"<% nvram_get_x(\"\", \"rt_TxPower\"); %>\">\n"
            "<input type=\"hidden\" name=\"rt_KickStaRssiLow\" value=\"<% nvram_get_x(\"\", \"rt_KickStaRssiLow\"); %>\">\n"
            "<input type=\"hidden\" name=\"rt_HT_BW\" value=\"<% nvram_get_x(\"\", \"rt_HT_BW\"); %>\">\n"
            "<input type=\"hidden\" name=\"rt_gmode\" value=\"<% nvram_get_x(\"\", \"rt_gmode\"); %>\">\n"
            "<input type=\"hidden\" name=\"rt_rrm_enable\" value=\"<% nvram_get_x(\"\", \"rt_rrm_enable\"); %>\">\n"
            "<input type=\"hidden\" name=\"rt_ft_enable\" value=\"<% nvram_get_x(\"\", \"rt_ft_enable\"); %>\">\n"
        ),
        "Advanced_WAdvanced2g_Content.asp": (
            "<input type=\"hidden\" name=\"rt_dtim\" value=\"<% nvram_get_x(\"\", \"rt_dtim\"); %>\">\n"
            "<input type=\"hidden\" name=\"rt_bcn\" value=\"<% nvram_get_x(\"\", \"rt_bcn\"); %>\">\n"
            "<input type=\"hidden\" name=\"rt_TxBurst\" value=\"<% nvram_get_x(\"\", \"rt_TxBurst\"); %>\">\n"
            "<input type=\"hidden\" name=\"rt_ap_isolate\" value=\"<% nvram_get_x(\"\", \"rt_ap_isolate\"); %>\">\n"
        ),
        "Advanced_FirmwareUpgrade_Content.asp": "<input type=\"hidden\" name=\"firmver\" value=\"<% nvram_get_x(\"\", \"firmver_sub\"); %>\">\n",
        "Advanced_System_Content.asp": "<input type=\"hidden\" name=\"computer_name2\" value=\"<% nvram_get_x(\"\", \"computer_name\"); %>\">\n",
        "Updating.asp": "OK: Updating\n",
        "UpdateError.asp": "UpdateError\n",
        "Uploading.asp": "OK: Uploading\n",
        "UploadDone.asp": "OK: UploadDone\n",
        "UploadError.asp": "UploadError\n",
        "Restarting.asp": "OK: Restarting\n",
    }
    for rel_path, content in headless_stubs.items():
        with open(os.path.join(min_www, rel_path), "w", encoding="utf-8", newline="\n") as f:
            f.write(content)

    # 6. Stage Concard N9 v2.1 hybrid CLI scripts & compile MIPS Zero-CPU DHCP Fingerprinter (concard-dhcp-fp)
    hybrid_bin_dir = os.path.join(TRUNK, "user/www/concard_hybrid_bins")
    if os.path.exists(hybrid_bin_dir):
        shutil.rmtree(hybrid_bin_dir)
    os.makedirs(hybrid_bin_dir, exist_ok=True)
    for bname in ("mca-dump", "concard-rrm-sync", "concard-vlan-apply", "concard-locate-led", "concard-watchdog", "concard-ota"):
        with open(os.path.join(WIN_BSP_DIR, bname), "r", encoding="utf-8") as bf:
            btxt = bf.read().replace("\r\n", "\n")
        bdest = os.path.join(hybrid_bin_dir, bname)
        with open(bdest, "w", encoding="utf-8", newline="\n") as bf:
            bf.write(btxt)
        os.chmod(bdest, 0o755)

    mips_gcc = os.path.join(os.path.dirname(TRUNK), "toolchain-mipsel/toolchain-3.4.x/bin/mipsel-linux-uclibc-gcc")
    assert os.path.isfile(mips_gcc), f"MIPS cross-compiler not found at {mips_gcc}"
    dhcp_fp_src = os.path.join(WIN_BSP_DIR, "concard_dhcp_fp.c")
    dhcp_fp_bin = os.path.join(hybrid_bin_dir, "concard-dhcp-fp")
    subprocess.run(
        [mips_gcc, "-Os", "-Wall", "-Wextra", "-s", "-o", dhcp_fp_bin, dhcp_fp_src],
        check=True,
    )
    os.chmod(dhcp_fp_bin, 0o755)
    print(f"[OK] Cross-compiled MIPS concard-dhcp-fp ({os.path.getsize(dhcp_fp_bin)} bytes)")

    www_mk_path = os.path.join(TRUNK, "user/www/Makefile")
    lean_www_mk = (
        "ifndef ROOTDIR\n"
        "ROOTDIR=../..\n"
        "endif\n\n"
        "ROMFS_DIR = $(ROOTDIR)/romfs\n\n"
        "# CONCARD_LEAN_WWW_NO_CLASSIC\n"
        "LABEL_COPYRIGHT_TEXT='Developed by <a href=\"https:\\/\\/viettechsoft.com\" target=\"_blank\" rel=\"noopener noreferrer\">Viettechsoft<\\/a>'\n\n"
        "all:\n\n"
        "romfs:\n"
        "\t@echo \"Make Concard N9 v2.1 Hybrid www, BPF OS Fingerprinter & GitHub OTA romfs\"\n"
        "\trm -Rf $(ROMFS_DIR)/www\n"
        "\tcp -R concard_minimal $(ROMFS_DIR)/www\n"
        "\tmkdir -p $(ROMFS_DIR)/usr/bin $(ROMFS_DIR)/usr/sbin\n"
        "\tcp -f concard_hybrid_bins/mca-dump $(ROMFS_DIR)/usr/bin/mca-dump\n"
        "\tcp -f concard_hybrid_bins/concard-rrm-sync $(ROMFS_DIR)/usr/sbin/concard-rrm-sync\n"
        "\tcp -f concard_hybrid_bins/concard-vlan-apply $(ROMFS_DIR)/usr/sbin/concard-vlan-apply\n"
        "\tcp -f concard_hybrid_bins/concard-locate-led $(ROMFS_DIR)/usr/sbin/concard-locate-led\n"
        "\tcp -f concard_hybrid_bins/concard-watchdog $(ROMFS_DIR)/usr/sbin/concard-watchdog\n"
        "\tcp -f concard_hybrid_bins/concard-dhcp-fp $(ROMFS_DIR)/usr/sbin/concard-dhcp-fp\n"
        "\tcp -f concard_hybrid_bins/concard-ota $(ROMFS_DIR)/usr/sbin/concard-ota\n"
        "\tchmod 0755 $(ROMFS_DIR)/usr/bin/mca-dump $(ROMFS_DIR)/usr/sbin/concard-rrm-sync $(ROMFS_DIR)/usr/sbin/concard-vlan-apply $(ROMFS_DIR)/usr/sbin/concard-locate-led $(ROMFS_DIR)/usr/sbin/concard-watchdog $(ROMFS_DIR)/usr/sbin/concard-dhcp-fp $(ROMFS_DIR)/usr/sbin/concard-ota\n\n"
        "clean:\n"
        "\t@echo \"Clean www romfs\"\n"
    )
    with open(www_mk_path, "w", encoding="utf-8", newline="\n") as f:
        f.write(lean_www_mk)
    print(f"[OK] Staged Concard N9 v2.1 web root & hybrid daemons -> {min_www}")


def patch_depmod_sh():
    dm_path = os.path.join(TRUNK, "tools/depmod.sh")
    with open(dm_path, "r", encoding="utf-8", errors="replace") as f:
        dm = f.read()
    changed = False
    old_call = '$depmod_bin -ae -F System.map -b "${INSTALL_MOD_PATH}" -r ${KERNELRELEASE}'
    new_call = '$depmod_bin -ae -F System.map -b "${INSTALL_MOD_PATH}" ${KERNELRELEASE}'
    if old_call in dm:
        dm = dm.replace(old_call, new_call, 1)
        changed = True
    if "modules.weakdep" not in dm:
        dm = dm.rstrip() + '\nrm -f "${INSTALL_MOD_PATH}/lib/modules/${KERNELRELEASE}/modules.weakdep"\n'
        changed = True
    if changed:
        with open(dm_path, "w", encoding="utf-8", newline="\n") as f:
            f.write(dm)
        print("[OK] Patched tools/depmod.sh (modern kmod compatibility without -r flag)")
    else:
        print("[OK] tools/depmod.sh already patched")


def main():
    os.makedirs(BOARD_DIR, exist_ok=True)
    os.makedirs(WIN_BSP_DIR, exist_ok=True)

    shutil.copyfile(os.path.join(WIN_BSP_DIR, "board.h"), os.path.join(BOARD_DIR, "board.h"))
    shutil.copyfile(os.path.join(WIN_BSP_DIR, "board.mk"), os.path.join(BOARD_DIR, "board.mk"))
    print(f"[OK] Installed board.h & board.mk -> {BOARD_DIR}")

    kcfg = build_kernel_config()
    with open(os.path.join(BOARD_DIR, "kernel-3.4.x.config"), "w", encoding="utf-8", newline="\n") as f:
        f.write(kcfg)
    with open(os.path.join(WIN_BSP_DIR, "kernel-3.4.x.config"), "w", encoding="utf-8", newline="\n") as f:
        f.write(kcfg)
    print(f"[OK] Generated kernel-3.4.x.config ({len(kcfg.splitlines())} lines)")

    tcfg = build_template_config()
    with open(TEMPLATE_PATH, "w", encoding="utf-8", newline="\n") as f:
        f.write(tcfg)
    with open(os.path.join(WIN_BSP_DIR, f"{BOARD_ID}.config"), "w", encoding="utf-8", newline="\n") as f:
        f.write(tcfg)
    print(f"[OK] Generated {TEMPLATE_PATH}")

    patch_ft_c()
    patch_rrm_neighbor_inject()
    patch_ralink_c()
    patch_defaults()
    patch_detect_link_c()
    patch_webui()
    patch_rust_wasm_webui()
    patch_mtd_storage()
    patch_dropbear_harden()
    patch_security_signature_lock()
    patch_depmod_sh()
    print("=== ALL CONCARD N9 v2.1 (CSB334A-2700-2100) BSP, KERNEL & RUST WASM UI PATCHES APPLIED SUCCESSFULLY ===")


if __name__ == "__main__":
    main()
