/*
 * Concard N9 v2.1 High-Speed C Telemetry, Deep Device Fingerprinting, Locate LED & GitHub OTA Engine
 * Executes in < 2ms directly inside Padavan httpd without fork/shell overhead on GET.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <ctype.h>
#include <unistd.h>
#include <sys/socket.h>
#include <sys/ioctl.h>
#include <net/if.h>
#include <net/if_arp.h>
#include <arpa/inet.h>

#include <iwlib.h>
#include "common.h"
#include <ralink_priv.h>
#include "httpd.h"
#include "concard_v2.h"

#ifndef RTPRIV_IOCTL_STATISTICS
#define RTPRIV_IOCTL_STATISTICS			(SIOCIWFIRSTPRIV + 0x09)
#endif
#ifndef RTPRIV_IOCTL_GET_MAC_TABLE_STRUCT
#define RTPRIV_IOCTL_GET_MAC_TABLE_STRUCT	(SIOCIWFIRSTPRIV + 0x1F)
#endif
#ifndef MTK_ESW_IOCTL_STATUS_SPEED_PORT
#define MTK_ESW_IOCTL_STATUS_SPEED_PORT		20
#endif

#define CONCARD_VERSION_STR		"Concard N9 v2.1"
#define CONCARD_BUILD_CODE		"CSB334A-2700-2100"
#define CONCARD_BUILD_DATE		"2026-11-19"
#define CONCARD_DEF_OTA_REPO		"https://github.com/fbaduongth/totolink_n9_firmware_custom"

#define CONCARD_LOCATE_UNTIL_FILE	"/tmp/.concard_locate_until"
#define CONCARD_BREATHING_FILE		"/tmp/.concard_breathing_active"
#define CONCARD_CTRL_URL_FILE		"/tmp/.concard_ctrl_url"
#define CONCARD_RRM_CNT_FILE		"/tmp/.concard_rrm_count"
#define CONCARD_GW_OK_FILE		"/tmp/.concard_gw_ok"
#define CONCARD_STA_FP_FILE		"/tmp/.concard_sta_fp"
#define CONCARD_OTA_CHECK_FILE		"/tmp/.concard_ota_check.json"

extern int wl_ioctl(const char *ifname, int cmd, struct iwreq *pwrq);
extern int mtk_esw_ioctl(unsigned int cmd, unsigned int par, unsigned int *value);

static char g_cached_ctrl_url[196] = {0};
static int g_locate_post_done = 0;
static char g_locate_post_json[256] = {0};
static int g_ota_post_done = 0;
static char g_ota_post_json[2048] = {0};

typedef struct {
	char mac[18];
	char ip[20];
	char host[64];
	char os_hint[44];
} sta_ip_map_t;

static void v2_json_escape(const char *src, char *dst, size_t dst_sz)
{
	size_t di = 0;
	const unsigned char *p = (const unsigned char *)(src ? src : "");
	if (dst_sz == 0)
		return;
	while (*p && di + 2 < dst_sz) {
		unsigned char c = *p++;
		if (c == '"' || c == '\\') {
			if (di + 3 >= dst_sz)
				break;
			dst[di++] = '\\';
			dst[di++] = (char)c;
		} else if (c >= 0x20 && c != 0x7F) {
			dst[di++] = (char)c;
		}
	}
	dst[di] = '\0';
}

static unsigned long read_proc_uptime_sec(void)
{
	FILE *fp = fopen("/proc/uptime", "r");
	unsigned long sec = 0;
	if (fp) {
		if (fscanf(fp, "%lu", &sec) != 1)
			sec = 0;
		fclose(fp);
	}
	return sec;
}

static unsigned long get_locate_remaining_sec(void)
{
	FILE *fp = fopen(CONCARD_LOCATE_UNTIL_FILE, "r");
	unsigned long until_sec = 0, now_sec;
	if (!fp)
		return 0;
	if (fscanf(fp, "%lu", &until_sec) != 1)
		until_sec = 0;
	fclose(fp);
	if (until_sec == 0)
		return 0;
	now_sec = read_proc_uptime_sec();
	return (until_sec > now_sec) ? (until_sec - now_sec) : 0;
}

static int read_small_int_file(const char *path, int def_val)
{
	FILE *fp = fopen(path, "r");
	int v = def_val;
	if (fp) {
		if (fscanf(fp, "%d", &v) != 1)
			v = def_val;
		fclose(fp);
	}
	return v;
}

static int nvram_get_int_def(const char *key, int def_val)
{
	const char *v = nvram_safe_get(key);
	if (!v || !v[0])
		return def_val;
	return atoi(v);
}

static int is_iface_alias_up(const char *ifname)
{
	int fd = socket(AF_INET, SOCK_DGRAM, 0);
	struct ifreq ifr;
	int up = 0;
	if (fd < 0)
		return 0;
	memset(&ifr, 0, sizeof(ifr));
	snprintf(ifr.ifr_name, sizeof(ifr.ifr_name), "%s", ifname);
	if (ioctl(fd, SIOCGIFFLAGS, &ifr) == 0 && (ifr.ifr_flags & IFF_UP)) {
		if (ioctl(fd, SIOCGIFADDR, &ifr) == 0) {
			struct sockaddr_in *sin = (struct sockaddr_in *)&ifr.ifr_addr;
			if (sin->sin_addr.s_addr != INADDR_ANY)
				up = 1;
		}
	}
	close(fd);
	return up;
}

/* ---------------- Controller Auto-Discovery Header Hook ---------------- */

void concard_v2_note_controller(const char *hdr_val)
{
	char clean[196];
	size_t i = 0;
	const char *p = hdr_val;
	const char *nv_cur;
	FILE *fp;

	if (!p)
		return;
	while (*p == ' ' || *p == '\t')
		p++;

	if (strncasecmp(p, "http://", 7) != 0 && strncasecmp(p, "https://", 8) != 0)
		return;

	while (*p && *p != '\r' && *p != '\n' && *p != ' ' && *p != '\t' && i + 1 < sizeof(clean)) {
		unsigned char c = (unsigned char)*p++;
		if (isalnum(c) || c == ':' || c == '/' || c == '.' || c == '-' || c == '_' || c == '[' || c == ']') {
			clean[i++] = (char)c;
		} else {
			return;
		}
	}
	while (i > 8 && clean[i - 1] == '/')
		i--;
	clean[i] = '\0';

	if (i < 10)
		return;

	if (strcmp(g_cached_ctrl_url, clean) != 0) {
		snprintf(g_cached_ctrl_url, sizeof(g_cached_ctrl_url), "%s", clean);
		fp = fopen(CONCARD_CTRL_URL_FILE, "w");
		if (fp) {
			fprintf(fp, "%s\n", clean);
			fclose(fp);
		}
	}

	nv_cur = nvram_safe_get("cc_ctrl_url");
	if (!nv_cur || strcmp(nv_cur, clean) != 0) {
		nvram_set("cc_ctrl_url", clean);
		nvram_commit_safe();
	}
}

/* ---------------- MCS / Rate Calculation (802.11b/g/n) ---------------- */

static const int v2_mcs_rate_tab[] = {
	/* 0..3: CCK */
	1, 2, 5, 11,
	/* 4..11: OFDM */
	6, 9, 12, 18, 24, 36, 48, 54,
	/* 12..43: 11n 20MHz LGI (MCS 0..31) */
	6,  13, 19,  26,  39,  52,  58,  65,
	13, 26, 39,  52,  78, 104, 117, 130,
	19, 39, 58,  78, 117, 156, 175, 195,
	26, 52, 78, 104, 156, 208, 234, 260,
	/* 44..75: 11n 40MHz LGI (MCS 0..31) */
	13, 27,  40,  54,  81, 108, 121, 135,
	27, 54,  81, 108, 162, 216, 243, 270,
	40, 81, 121, 162, 243, 324, 364, 405,
	54, 108, 162, 216, 324, 432, 486, 540,
	/* 76..107: 11n 20MHz SGI (MCS 0..31) */
	7,  14, 21,  28,  43,  57,  65,  72,
	14, 28, 43,  57,  86, 115, 130, 144,
	21, 43, 65,  86, 130, 173, 195, 216,
	28, 57, 86, 115, 173, 231, 260, 288,
	/* 108..139: 11n 40MHz SGI (MCS 0..31) */
	15, 30,  45,  60,  90, 120, 135, 150,
	30, 60,  90, 120, 180, 240, 270, 300,
	45, 90, 135, 180, 270, 360, 405, 450,
	60, 120, 180, 240, 360, 480, 540, 600
};

static int v2_legacy_ofdm_idx(unsigned int mcs)
{
	switch (mcs) {
	case 0xb: return 0;
	case 0xf: return 1;
	case 0xa: return 2;
	case 0xe: return 3;
	case 0x9: return 4;
	case 0xd: return 5;
	case 0x8: return 6;
	case 0xc: return 7;
	default:  return (mcs < 8) ? (int)mcs : 7;
	}
}

static int v2_calc_rate_mbps(MACHTTRANSMIT_SETTING ht)
{
	int idx = 0;
	int tab_len = (int)(sizeof(v2_mcs_rate_tab) / sizeof(v2_mcs_rate_tab[0]));
	unsigned int mode = ht.field.MODE;
	unsigned int bw = (ht.field.BW == BW_40) ? 1 : 0;
	unsigned int sgi = ht.field.ShortGI ? 1 : 0;
	unsigned int mcs = ht.field.MCS;

	if (mode >= MODE_HTMIX) {
		if (mcs > 31)
			mcs &= 0x1F;
		idx = 12 + (int)(bw * 32) + (int)(sgi * 64) + (int)mcs;
	} else if (mode == MODE_OFDM) {
		idx = 4 + v2_legacy_ofdm_idx(mcs);
	} else {
		idx = (int)(mcs & 0x03);
	}

	if (idx < 0)
		idx = 0;
	if (idx >= tab_len)
		idx = tab_len - 1;
	return v2_mcs_rate_tab[idx];
}

/* ---------------- System, Network & Wireless Readers ---------------- */

static void read_loadavg(char *out, size_t out_sz)
{
	FILE *fp = fopen("/proc/loadavg", "r");
	char a[16] = "0.00", b[16] = "0.00", c[16] = "0.00";
	if (fp) {
		if (fscanf(fp, "%15s %15s %15s", a, b, c) < 3) {
			strcpy(a, "0.00"); strcpy(b, "0.00"); strcpy(c, "0.00");
		}
		fclose(fp);
	}
	snprintf(out, out_sz, "%s %s %s", a, b, c);
}

static void read_meminfo_kb(unsigned long *total_kb, unsigned long *used_kb, unsigned long *free_kb)
{
	FILE *fp = fopen("/proc/meminfo", "r");
	char line[128], key[48];
	unsigned long val, mem_total = 0, mem_free = 0, buffers = 0, cached = 0, avail;

	if (fp) {
		while (fgets(line, sizeof(line), fp)) {
			if (sscanf(line, "%47[^:]: %lu", key, &val) == 2) {
				if (strcmp(key, "MemTotal") == 0)
					mem_total = val;
				else if (strcmp(key, "MemFree") == 0)
					mem_free = val;
				else if (strcmp(key, "Buffers") == 0)
					buffers = val;
				else if (strcmp(key, "Cached") == 0)
					cached = val;
			}
		}
		fclose(fp);
	}
	avail = mem_free + buffers + cached;
	if (avail > mem_total)
		avail = mem_total;
	*total_kb = mem_total;
	*free_kb = avail;
	*used_kb = (mem_total >= avail) ? (mem_total - avail) : 0;
}

static void read_netdev_bytes(unsigned long long *wlan_rx, unsigned long long *wlan_tx,
                              unsigned long long *lan_rx, unsigned long long *lan_tx)
{
	FILE *fp = fopen("/proc/net/dev", "r");
	char line[256];
	*wlan_rx = 0; *wlan_tx = 0;
	*lan_rx = 0;  *lan_tx = 0;
	if (!fp)
		return;
	while (fgets(line, sizeof(line), fp)) {
		char *colon = strchr(line, ':');
		char ifn[32];
		unsigned long long rx_b = 0, tx_b = 0, d1, d2, d3, d4, d5, d6, d7;
		const char *p;
		size_t nlen;
		if (!colon)
			continue;
		p = line;
		while (*p == ' ' || *p == '\t')
			p++;
		nlen = (size_t)(colon - p);
		if (nlen == 0 || nlen >= sizeof(ifn))
			continue;
		memcpy(ifn, p, nlen);
		ifn[nlen] = '\0';
		if (sscanf(colon + 1, "%llu %llu %llu %llu %llu %llu %llu %llu %llu",
		           &rx_b, &d1, &d2, &d3, &d4, &d5, &d6, &d7, &tx_b) == 9) {
			if (strcmp(ifn, "ra0") == 0) {
				*wlan_rx = rx_b;
				*wlan_tx = tx_b;
			} else if (strcmp(ifn, "eth2") == 0) {
				*lan_rx = rx_b;
				*lan_tx = tx_b;
			}
		}
	}
	fclose(fp);
}

static void read_iface_mac(const char *ifname, char *out, size_t out_sz)
{
	char path[64], buf[32];
	FILE *fp;
	size_t i;
	out[0] = '\0';
	snprintf(path, sizeof(path), "/sys/class/net/%s/address", ifname);
	fp = fopen(path, "r");
	if (!fp)
		return;
	if (fgets(buf, sizeof(buf), fp)) {
		for (i = 0; buf[i] && i + 1 < out_sz; i++) {
			if (buf[i] == '\r' || buf[i] == '\n')
				break;
			out[i] = (char)toupper((unsigned char)buf[i]);
		}
		out[i] = '\0';
	}
	fclose(fp);
}

static void read_live_br0_ip(char *out, size_t out_sz)
{
	int fd = socket(AF_INET, SOCK_DGRAM, 0);
	struct ifreq ifr;
	const char *nv_ip;
	if (fd >= 0) {
		memset(&ifr, 0, sizeof(ifr));
		snprintf(ifr.ifr_name, sizeof(ifr.ifr_name), "br0");
		if (ioctl(fd, SIOCGIFADDR, &ifr) == 0) {
			struct sockaddr_in *sin = (struct sockaddr_in *)&ifr.ifr_addr;
			if (sin->sin_addr.s_addr != INADDR_ANY) {
				snprintf(out, out_sz, "%s", inet_ntoa(sin->sin_addr));
				close(fd);
				return;
			}
		}
		close(fd);
	}
	nv_ip = nvram_safe_get("lan_ipaddr_t");
	if (!nv_ip || !nv_ip[0] || strcmp(nv_ip, "0.0.0.0") == 0)
		nv_ip = nvram_safe_get("lan_ipaddr");
	snprintf(out, out_sz, "%s", nv_ip ? nv_ip : "");
}

static int read_live_ra0_channel(int fallback_ch)
{
	struct iwreq wrq;
	memset(&wrq, 0, sizeof(wrq));
	if (wl_ioctl("ra0", SIOCGIWFREQ, &wrq) >= 0) {
		double f = iw_freq2float(&(wrq.u.freq));
		if (f >= 1.0 && f <= 14.0)
			return (int)(f + 0.5);
		if (f >= 2412000000.0 && f <= 2484000000.0) {
			int mhz = (int)(f / 1000000.0 + 0.5);
			if (mhz == 2484)
				return 14;
			return (mhz - 2407) / 5;
		}
		if (f >= 2412.0 && f <= 2484.0) {
			int mhz = (int)(f + 0.5);
			if (mhz == 2484)
				return 14;
			return (mhz - 2407) / 5;
		}
	}
	return fallback_ch;
}

static void parse_ra0_statistics(int *ch_busy_pct, unsigned long *false_cca_1s,
                                 unsigned long *false_cca_tot, int *temp_c, int *avg_rssi)
{
	char stat_buf[4096];
	struct iwreq wrq;
	char *saveptr = NULL, *line;
	int got_1s = 0;

	*ch_busy_pct = 0;
	*false_cca_1s = 0;
	*false_cca_tot = 0;
	*temp_c = 0;
	*avg_rssi = 0;

	memset(stat_buf, 0, sizeof(stat_buf));
	memset(&wrq, 0, sizeof(wrq));
	wrq.u.data.pointer = stat_buf;
	wrq.u.data.length = sizeof(stat_buf) - 1;
	wrq.u.data.flags = 0;

	if (wl_ioctl("ra0", RTPRIV_IOCTL_STATISTICS, &wrq) < 0)
		return;

	stat_buf[sizeof(stat_buf) - 1] = '\0';
	for (line = strtok_r(stat_buf, "\r\n", &saveptr); line != NULL; line = strtok_r(NULL, "\r\n", &saveptr)) {
		char *eq = strchr(line, '=');
		const char *val_s;
		if (!eq)
			continue;
		val_s = eq + 1;
		while (*val_s == ' ' || *val_s == '\t')
			val_s++;

		if (strncmp(line, "Channel busy", 12) == 0) {
			int v = atoi(val_s);
			if (v < 0) v = 0;
			if (v > 100) v = 100;
			*ch_busy_pct = v;
		} else if (strncmp(line, "False CCA (1s)", 14) == 0 || strncmp(line, "False CCA (one second)", 22) == 0) {
			*false_cca_1s = strtoul(val_s, NULL, 10);
			got_1s = 1;
		} else if (strncmp(line, "False CCA", 9) == 0) {
			*false_cca_tot = strtoul(val_s, NULL, 10);
		} else if (strncmp(line, "Current temperature", 19) == 0) {
			*temp_c = atoi(val_s);
		} else if (strncmp(line, "Average RSSI", 12) == 0) {
			*avg_rssi = atoi(val_s);
		}
	}

	if (!got_1s)
		*false_cca_1s = *false_cca_tot;
}

static int find_sta_map_idx(sta_ip_map_t *map, int cnt, const char *mac)
{
	int i;
	for (i = 0; i < cnt; i++) {
		if (strcmp(map[i].mac, mac) == 0)
			return i;
	}
	return -1;
}

static int load_sta_ip_map(sta_ip_map_t *map, int max_items)
{
	int cnt = 0;
	FILE *fp;
	char line[256];

	/* 1. Load live BPF DHCP fingerprint & Option 12/50 table (/tmp/.concard_sta_fp) */
	fp = fopen(CONCARD_STA_FP_FILE, "r");
	if (fp) {
		while (cnt < max_items && fgets(line, sizeof(line), fp)) {
			char *nl = strpbrk(line, "\r\n");
			char *t1, *t2, *t3;
			size_t k;
			if (nl) *nl = '\0';
			t1 = strchr(line, '\t');
			if (!t1 || (size_t)(t1 - line) != 17)
				continue;
			*t1 = '\0';
			t2 = strchr(t1 + 1, '\t');
			if (!t2)
				continue;
			*t2 = '\0';
			t3 = strchr(t2 + 1, '\t');
			if (t3)
				*t3 = '\0';

			memset(&map[cnt], 0, sizeof(map[cnt]));
			for (k = 0; k < 17; k++)
				map[cnt].mac[k] = (char)toupper((unsigned char)line[k]);
			map[cnt].mac[17] = '\0';
			snprintf(map[cnt].ip, sizeof(map[cnt].ip), "%s", t1 + 1);
			snprintf(map[cnt].os_hint, sizeof(map[cnt].os_hint), "%s", t2 + 1);
			if (t3)
				snprintf(map[cnt].host, sizeof(map[cnt].host), "%s", t3 + 1);
			cnt++;
		}
		fclose(fp);
	}

	/* 2. Merge Padavan networkmap (/tmp/static_ip.inf) */
	fp = fopen("/tmp/static_ip.inf", "r");
	if (fp) {
		while (fgets(line, sizeof(line), fp)) {
			char *c1 = strchr(line, ',');
			char *c2 = c1 ? strchr(c1 + 1, ',') : NULL;
			char *c3 = c2 ? strchr(c2 + 1, ',') : NULL;
			size_t ip_len, mac_len, h_len, k;
			char mac_s[18], ip_s[20], host_s[64];
			int idx;
			if (!c1 || !c2)
				continue;
			ip_len = (size_t)(c1 - line);
			mac_len = (size_t)(c2 - (c1 + 1));
			if (ip_len == 0 || ip_len >= sizeof(ip_s) || mac_len != 17)
				continue;
			memcpy(ip_s, line, ip_len);
			ip_s[ip_len] = '\0';
			for (k = 0; k < 17; k++)
				mac_s[k] = (char)toupper((unsigned char)c1[1 + k]);
			mac_s[17] = '\0';
			host_s[0] = '\0';
			if (c3 && c3 > c2 + 1) {
				h_len = (size_t)(c3 - (c2 + 1));
				if (h_len >= sizeof(host_s))
					h_len = sizeof(host_s) - 1;
				memcpy(host_s, c2 + 1, h_len);
				host_s[h_len] = '\0';
				if (strcmp(host_s, "null") == 0 || strcmp(host_s, "*") == 0)
					host_s[0] = '\0';
			}
			idx = find_sta_map_idx(map, cnt, mac_s);
			if (idx >= 0) {
				if (ip_s[0] && strcmp(ip_s, "0.0.0.0") != 0)
					snprintf(map[idx].ip, sizeof(map[idx].ip), "%s", ip_s);
				if (!map[idx].host[0] && host_s[0])
					snprintf(map[idx].host, sizeof(map[idx].host), "%s", host_s);
			} else if (cnt < max_items) {
				memset(&map[cnt], 0, sizeof(map[cnt]));
				snprintf(map[cnt].mac, sizeof(map[cnt].mac), "%s", mac_s);
				snprintf(map[cnt].ip, sizeof(map[cnt].ip), "%s", ip_s);
				snprintf(map[cnt].host, sizeof(map[cnt].host), "%s", host_s);
				cnt++;
			}
		}
		fclose(fp);
	}

	/* 3. Supplement with live /proc/net/arp entries */
	fp = fopen("/proc/net/arp", "r");
	if (fp) {
		if (fgets(line, sizeof(line), fp)) {
			while (fgets(line, sizeof(line), fp)) {
				char ip_s[32], hw_s[16], flg_s[16], mac_s[32];
				unsigned long flg;
				size_t k;
				int idx;
				if (sscanf(line, "%31s %15s %15s %31s", ip_s, hw_s, flg_s, mac_s) != 4)
					continue;
				flg = strtoul(flg_s, NULL, 0);
				if (flg == 0 || strlen(mac_s) != 17)
					continue;
				for (k = 0; k < 17; k++)
					mac_s[k] = (char)toupper((unsigned char)mac_s[k]);
				idx = find_sta_map_idx(map, cnt, mac_s);
				if (idx >= 0) {
					if (!map[idx].ip[0] || strcmp(map[idx].ip, "0.0.0.0") == 0)
						snprintf(map[idx].ip, sizeof(map[idx].ip), "%s", ip_s);
				} else if (cnt < max_items) {
					memset(&map[cnt], 0, sizeof(map[cnt]));
					snprintf(map[cnt].mac, sizeof(map[cnt].mac), "%s", mac_s);
					snprintf(map[cnt].ip, sizeof(map[cnt].ip), "%s", ip_s);
					cnt++;
				}
			}
		}
		fclose(fp);
	}

	return cnt;
}

static void resolve_station_os_hint(
	const unsigned char addr[6],
	unsigned int assoc_vendor_code,
	const char *dhcp_os,
	const char *host,
	char *out_os,
	size_t out_sz)
{
	out_os[0] = '\0';

	/* Fuse Layer 1 (802.11 Assoc Vendor IE) + Layer 2 (DHCP Option 55/60) */
	if (assoc_vendor_code == 2) {
		/* Samsung 00:00:F0 IE confirmed in 802.11 AssocReq */
		if (dhcp_os && strncasecmp(dhcp_os, "Android", 7) == 0)
			snprintf(out_os, out_sz, "Samsung (%s)", dhcp_os);
		else if (dhcp_os && dhcp_os[0])
			snprintf(out_os, out_sz, "%s", dhcp_os);
		else
			snprintf(out_os, out_sz, "Samsung Galaxy");
		return;
	}

	if (dhcp_os && dhcp_os[0]) {
		if (assoc_vendor_code == 6 && strcmp(dhcp_os, "Apple iOS") == 0)
			snprintf(out_os, out_sz, "Apple macOS");
		else
			snprintf(out_os, out_sz, "%s", dhcp_os);
		return;
	}

	switch (assoc_vendor_code) {
	case 1:
		snprintf(out_os, out_sz, "Apple iOS");
		return;
	case 6:
		snprintf(out_os, out_sz, "Apple macOS");
		return;
	case 3:
		snprintf(out_os, out_sz, "Windows PC");
		return;
	case 4:
	case 5:
		snprintf(out_os, out_sz, "Android");
		return;
	default:
		break;
	}

	if (host && host[0]) {
		if (strncasecmp(host, "iphone", 6) == 0) {
			snprintf(out_os, out_sz, "Apple iOS");
			return;
		}
		if (strncasecmp(host, "ipad", 4) == 0) {
			snprintf(out_os, out_sz, "Apple iPadOS");
			return;
		}
		if (strncasecmp(host, "macbook", 7) == 0 || strncasecmp(host, "imac", 4) == 0) {
			snprintf(out_os, out_sz, "Apple macOS");
			return;
		}
		if (strncasecmp(host, "desktop-", 8) == 0 || strncasecmp(host, "laptop-", 7) == 0) {
			snprintf(out_os, out_sz, "Windows PC");
			return;
		}
	}

	/* Distinguish Randomized LAA MAC vs IEEE Global MAC when no Vendor IE / DHCP seen yet */
	if (addr[0] & 0x02)
		snprintf(out_os, out_sz, "Private MAC");
}

/* ---------------- GET /api_telemetry.cgi ---------------- */

void do_concard_telemetry_cgi(const char *url, FILE *stream)
{
	static const char *port_labels[5] = { "LAN / PoE", "LAN1", "LAN2", "LAN3", "LAN4" };
	unsigned long uptime_sec, mem_tot_kb = 0, mem_used_kb = 0, mem_free_kb = 0;
	unsigned long false_cca_1s = 0, false_cca_tot = 0, locate_rem = 0;
	unsigned long long wlan_rx = 0, wlan_tx = 0, lan_rx = 0, lan_tx = 0;
	char load_avg[48], live_ip[32], ap_mac[20], guest_mac[20];
	char e_fw[96], e_hn[96], e_loc[160], e_ip[48], e_gw[48], e_mask[48], e_ssid[96], e_gssid[96];
	char e_ctrl[220], e_ota[256];
	const char *fw, *hn, *loc, *gw_t, *mask_t, *proto_x, *ssid, *gssid, *ctrl_u, *ota_u;
	int rt_radio, rt_ch_cfg, live_ch, rt_bw, rt_txp, rt_rrm, rt_ft, cell_br, rt_gmode, guest_en, guest_vlan;
	int cc_qload, cc_probe_rssi, cc_fair_q, cc_l2_shield, cc_led_mode;
	int ch_busy_pct = 0, temp_c = 0, avg_rssi = 0, breathing_active = 0, rrm_cnt = 0;
	int link_up = 0, max_speed = 0, degraded_10m = 0, gw_ok = 1, fb_alias = 0, p;
	int p_up[5], p_spd[5];
	const char *p_dup[5];
	sta_ip_map_t ip_map[128];
	int ip_map_cnt;
	char mac_buf[sizeof(RT_802_11_MAC_TABLE) + 64];
	struct iwreq wrq;
	RT_802_11_MAC_TABLE *mp = NULL;
	unsigned long sta_total = 0, idx;

	(void)url;

	uptime_sec = read_proc_uptime_sec();
	read_loadavg(load_avg, sizeof(load_avg));
	read_meminfo_kb(&mem_tot_kb, &mem_used_kb, &mem_free_kb);
	read_netdev_bytes(&wlan_rx, &wlan_tx, &lan_rx, &lan_tx);

	fw = nvram_safe_get("firmver_sub");
	if (!fw || !fw[0])
		fw = "3.4.3.9-099_Concard-N9-v2.1";
	hn = nvram_safe_get("computer_name");
	if (!hn || !hn[0])
		hn = "Concard-N9";
	loc = nvram_safe_get("cc_device_loc");

	read_live_br0_ip(live_ip, sizeof(live_ip));
	gw_t = nvram_safe_get("lan_gateway_t");
	if (!gw_t || !gw_t[0] || strcmp(gw_t, "0.0.0.0") == 0)
		gw_t = nvram_safe_get("lan_gateway");
	mask_t = nvram_safe_get("lan_netmask_t");
	if (!mask_t || !mask_t[0] || strcmp(mask_t, "0.0.0.0") == 0)
		mask_t = nvram_safe_get("lan_netmask");
	proto_x = nvram_safe_get("lan_proto_x");

	rt_radio = atoi(nvram_safe_get("rt_radio_x"));
	ssid = nvram_safe_get("rt_ssid");
	rt_ch_cfg = atoi(nvram_safe_get("rt_channel"));
	live_ch = (rt_radio != 0) ? read_live_ra0_channel(rt_ch_cfg) : rt_ch_cfg;
	rt_bw = atoi(nvram_safe_get("rt_HT_BW"));
	rt_txp = atoi(nvram_safe_get("rt_TxPower"));
	rt_rrm = atoi(nvram_safe_get("rt_rrm_enable"));
	rt_ft = atoi(nvram_safe_get("rt_ft_enable"));
	cell_br = nvram_get_int_def("cc_cell_breathing", 1);
	rt_gmode = nvram_get_int_def("rt_gmode", 5);
	guest_en = atoi(nvram_safe_get("rt_guest_enable"));
	gssid = nvram_safe_get("rt_guest_ssid");
	guest_vlan = atoi(nvram_safe_get("cc_guest_vlan"));

	cc_qload = nvram_get_int_def("cc_qload_enable", 1);
	cc_probe_rssi = nvram_get_int_def("cc_probe_rssi", -82);
	cc_fair_q = nvram_get_int_def("cc_fair_queue", 1);
	cc_l2_shield = nvram_get_int_def("cc_l2_shield", 1);
	cc_led_mode = nvram_get_int_def("cc_led_mode", 0);

	ctrl_u = nvram_safe_get("cc_ctrl_url");
	ota_u = nvram_safe_get("cc_ota_repo_url");
	if (!ota_u || !ota_u[0])
		ota_u = CONCARD_DEF_OTA_REPO;

	read_iface_mac("ra0", ap_mac, sizeof(ap_mac));
	if (!ap_mac[0])
		read_iface_mac("br0", ap_mac, sizeof(ap_mac));
	guest_mac[0] = '\0';
	if (guest_en)
		read_iface_mac("ra1", guest_mac, sizeof(guest_mac));

	/* Physical Ethernet Port Status (Port 0 = Single Physical LAN/PoE Jack on N9v1) */
	for (p = 0; p < 5; p++) {
		unsigned int val = 0;
		p_up[p] = 0;
		p_spd[p] = 0;
		p_dup[p] = "NONE";
		if (mtk_esw_ioctl(MTK_ESW_IOCTL_STATUS_SPEED_PORT, (unsigned int)p, &val) == 0) {
			if ((val >> 16) & 0x01) {
				int spd = 10;
				int fd_bit = (int)((val >> 8) & 0x01);
				switch (val & 0x03) {
				case 3:
				case 2: spd = 1000; break;
				case 1: spd = 100;  break;
				default: spd = 10;  break;
				}
				p_up[p] = 1;
				p_spd[p] = spd;
				p_dup[p] = fd_bit ? "FD" : "HD";
				link_up = 1;
				if (spd > max_speed)
					max_speed = spd;
				if (spd == 10 || !fd_bit)
					degraded_10m = 1;
			}
		}
	}

	gw_ok = read_small_int_file(CONCARD_GW_OK_FILE, link_up ? 1 : 0) ? 1 : 0;
	if (!link_up)
		gw_ok = 0;
	fb_alias = is_iface_alias_up("br0:fb");

	if (rt_radio != 0)
		parse_ra0_statistics(&ch_busy_pct, &false_cca_1s, &false_cca_tot, &temp_c, &avg_rssi);

	locate_rem = get_locate_remaining_sec();
	breathing_active = read_small_int_file(CONCARD_BREATHING_FILE, 0) ? 1 : 0;
	rrm_cnt = read_small_int_file(CONCARD_RRM_CNT_FILE, 0);

	ip_map_cnt = load_sta_ip_map(ip_map, (int)(sizeof(ip_map) / sizeof(ip_map[0])));

	memset(mac_buf, 0, sizeof(mac_buf));
	memset(&wrq, 0, sizeof(wrq));
	wrq.u.data.pointer = mac_buf;
	wrq.u.data.length = sizeof(RT_802_11_MAC_TABLE);
	wrq.u.data.flags = 0;
	if (rt_radio != 0 && wl_ioctl("ra0", RTPRIV_IOCTL_GET_MAC_TABLE_STRUCT, &wrq) >= 0) {
		mp = (RT_802_11_MAC_TABLE *)wrq.u.data.pointer;
		sta_total = mp->Num;
		if (sta_total > MAX_NUMBER_OF_MAC)
			sta_total = MAX_NUMBER_OF_MAC;
	}

	v2_json_escape(fw, e_fw, sizeof(e_fw));
	v2_json_escape(hn, e_hn, sizeof(e_hn));
	v2_json_escape(loc, e_loc, sizeof(e_loc));
	v2_json_escape(live_ip, e_ip, sizeof(e_ip));
	v2_json_escape(gw_t, e_gw, sizeof(e_gw));
	v2_json_escape(mask_t, e_mask, sizeof(e_mask));
	v2_json_escape(ssid, e_ssid, sizeof(e_ssid));
	v2_json_escape(gssid, e_gssid, sizeof(e_gssid));
	v2_json_escape(ctrl_u, e_ctrl, sizeof(e_ctrl));
	v2_json_escape(ota_u, e_ota, sizeof(e_ota));

	fprintf(stream,
		"{\"ok\":true,\"version\":\"%s\",\"buildCode\":\"%s\",\"buildDate\":\"%s\",\"firmver_sub\":\"%s\","
		"\"computer_name\":\"%s\",\"hostname\":\"%s\",\"cc_device_loc\":\"%s\",\"location\":\"%s\","
		"\"apMac\":\"%s\",\"guestMac\":\"%s\","
		"\"lan_ipaddr_t\":\"%s\",\"lan_netmask_t\":\"%s\",\"lan_gateway_t\":\"%s\",\"lan_proto_x\":%d,"
		"\"gatewayReachable\":%s,\"fallbackAliasActive\":%s,"
		"\"uptimeSec\":%lu,\"loadAvg\":\"%s\",\"memTotalKb\":%lu,\"memUsedKb\":%lu,\"memFreeKb\":%lu,"
		"\"rt_radio_x\":%d,\"rt_ssid\":\"%s\",\"rt_channel\":%d,\"liveChannel\":%d,"
		"\"rt_HT_BW\":%d,\"rt_TxPower\":%d,\"rt_rrm_enable\":%d,\"rt_ft_enable\":%d,"
		"\"cc_cell_breathing\":%d,\"rt_gmode\":%d,"
		"\"cc_qload_enable\":%d,\"cc_probe_rssi\":%d,\"cc_fair_queue\":%d,\"cc_l2_shield\":%d,\"cc_led_mode\":%d,"
		"\"cc_ctrl_url\":\"%s\",\"cc_ota_repo_url\":\"%s\","
		"\"rt_guest_enable\":%d,\"rt_guest_ssid\":\"%s\",\"cc_guest_vlan\":%d,"
		"\"linkUp\":%s,\"maxSpeedMbps\":%d,\"degraded10M\":%s,\"ports\":[",
		CONCARD_VERSION_STR, CONCARD_BUILD_CODE, CONCARD_BUILD_DATE, e_fw,
		e_hn, e_hn, e_loc, e_loc,
		ap_mac, guest_mac,
		e_ip, e_mask, e_gw, (*proto_x == '0') ? 0 : 1,
		gw_ok ? "true" : "false", fb_alias ? "true" : "false",
		uptime_sec, load_avg, mem_tot_kb, mem_used_kb, mem_free_kb,
		rt_radio, e_ssid, rt_ch_cfg, live_ch,
		rt_bw, rt_txp, rt_rrm, rt_ft,
		cell_br, rt_gmode,
		cc_qload ? 1 : 0, cc_probe_rssi, cc_fair_q ? 1 : 0, cc_l2_shield ? 1 : 0, cc_led_mode,
		e_ctrl, e_ota,
		guest_en, e_gssid, guest_vlan,
		link_up ? "true" : "false", max_speed, degraded_10m ? "true" : "false");

	for (p = 0; p < 5; p++) {
		fprintf(stream,
			"%s{\"id\":%d,\"label\":\"%s\",\"up\":%s,\"speed\":%d,\"duplex\":\"%s\"}",
			(p == 0) ? "" : ",",
			p, port_labels[p], p_up[p] ? "true" : "false", p_spd[p], p_dup[p]);
	}

	fprintf(stream,
		"],\"wlanRx\":%llu,\"wlanTx\":%llu,\"lanRx\":%llu,\"lanTx\":%llu,"
		"\"channelBusyPct\":%d,\"falseCca1s\":%lu,\"falseCca\":%lu,\"tempC\":%d,\"avgRssi\":%d,"
		"\"locateRemainingSec\":%lu,\"breathingActive\":%d,\"rrmCount\":%d,"
		"\"staCount\":%lu,\"stations\":[",
		wlan_rx, wlan_tx, lan_rx, lan_tx,
		ch_busy_pct, false_cca_1s, false_cca_tot, temp_c, avg_rssi,
		locate_rem, breathing_active, rrm_cnt,
		sta_total);

	for (idx = 0; mp != NULL && idx < sta_total; idx++) {
		RT_802_11_MAC_ENTRY *e = &mp->Entry[idx];
		char mac_s[18], os_raw[44], e_sip[32], e_shost[96], e_sos[64];
		const char *mode_s, *found_ip = "", *found_host = "", *found_dhcp_os = "";
		unsigned int vendor_code = (unsigned int)((e->LastRxRate >> 16) & 0xFFU);
		unsigned short raw_rx_word = (unsigned short)(e->LastRxRate & 0xFFFFU);
		int r0 = (int)e->AvgRssi0;
		int r1 = (int)e->AvgRssi1;
		int best_rssi, tx_mbps, rx_mbps, bw_mhz, m_idx;
		MACHTTRANSMIT_SETTING rx_ht;

		snprintf(mac_s, sizeof(mac_s), "%02X:%02X:%02X:%02X:%02X:%02X",
			e->Addr[0], e->Addr[1], e->Addr[2], e->Addr[3], e->Addr[4], e->Addr[5]);

		if (r0 != 0 && r1 != 0)
			best_rssi = (r0 > r1) ? r0 : r1;
		else if (r0 != 0)
			best_rssi = r0;
		else if (r1 != 0)
			best_rssi = r1;
		else
			best_rssi = -70;

		tx_mbps = v2_calc_rate_mbps(e->TxRate);
		if (raw_rx_word != 0) {
			rx_ht.word = raw_rx_word;
			rx_mbps = v2_calc_rate_mbps(rx_ht);
		} else {
			rx_mbps = tx_mbps;
		}
		bw_mhz = (e->TxRate.field.BW == BW_40) ? 40 : 20;
		if (e->TxRate.field.MODE == MODE_CCK)
			mode_s = "802.11b";
		else if (e->TxRate.field.MODE == MODE_OFDM)
			mode_s = "802.11g";
		else
			mode_s = "802.11n";

		m_idx = find_sta_map_idx(ip_map, ip_map_cnt, mac_s);
		if (m_idx >= 0) {
			found_ip = ip_map[m_idx].ip;
			found_host = ip_map[m_idx].host;
			found_dhcp_os = ip_map[m_idx].os_hint;
		}

		resolve_station_os_hint(e->Addr, vendor_code, found_dhcp_os, found_host, os_raw, sizeof(os_raw));
		v2_json_escape(found_ip, e_sip, sizeof(e_sip));
		v2_json_escape(found_host, e_shost, sizeof(e_shost));
		v2_json_escape(os_raw, e_sos, sizeof(e_sos));

		fprintf(stream,
			"%s{\"mac\":\"%s\",\"apIdx\":%u,\"ip\":\"%s\",\"hostname\":\"%s\",\"osHint\":\"%s\","
			"\"rssi\":%d,\"rssi0\":%d,\"rssi1\":%d,"
			"\"txMbps\":%d,\"rxMbps\":%d,\"bw\":%d,\"mode\":\"%s\","
			"\"psm\":%u,\"connSec\":%u}",
			(idx == 0) ? "" : ",",
			mac_s, (unsigned int)e->ApIdx, e_sip, e_shost, e_sos,
			best_rssi, r0, r1,
			tx_mbps, rx_mbps, bw_mhz, mode_s,
			(unsigned int)e->Psm, (unsigned int)e->ConnectedTime);
	}

	fprintf(stream, "]}\n");
	fflush(stream);
}

/* ---------------- POST Body & URL-Encoded Helpers ---------------- */

static void drain_and_read_body(FILE *stream, int clen, char *buf, size_t buf_sz)
{
	size_t want, got = 0;
	if (buf_sz == 0)
		return;
	buf[0] = '\0';
	if (clen <= 0)
		return;
	want = (size_t)clen;
	if (want >= buf_sz)
		want = buf_sz - 1;
	while (got < want) {
		size_t r = fread(buf + got, 1, want - got, stream);
		if (r == 0)
			break;
		got += r;
	}
	buf[got] = '\0';
	if ((size_t)clen > got) {
		size_t rem = (size_t)clen - got;
		char tmp[128];
		while (rem > 0) {
			size_t chunk = (rem > sizeof(tmp)) ? sizeof(tmp) : rem;
			if (fread(tmp, 1, chunk, stream) == 0)
				break;
			rem -= chunk;
		}
	}
}

static int v2_hex_nib(char c)
{
	if (c >= '0' && c <= '9') return c - '0';
	if (c >= 'a' && c <= 'f') return c - 'a' + 10;
	if (c >= 'A' && c <= 'F') return c - 'A' + 10;
	return -1;
}

static void v2_url_decode(const char *src, size_t len, char *dst, size_t dst_sz)
{
	size_t si = 0, di = 0;
	if (dst_sz == 0)
		return;
	while (si < len && di + 1 < dst_sz) {
		char ch = src[si];
		if (ch == '+') {
			dst[di++] = ' ';
			si++;
		} else if (ch == '%' && si + 2 < len) {
			int hi = v2_hex_nib(src[si + 1]);
			int lo = v2_hex_nib(src[si + 2]);
			if (hi >= 0 && lo >= 0) {
				dst[di++] = (char)((hi << 4) | lo);
				si += 3;
				continue;
			}
			dst[di++] = ch;
			si++;
		} else {
			dst[di++] = ch;
			si++;
		}
	}
	dst[di] = '\0';
}

static void v2_form_param(const char *body, const char *key, char *out, size_t out_sz)
{
	const char *p = body ? body : "";
	if (out_sz == 0)
		return;
	out[0] = '\0';
	while (*p) {
		const char *amp = strchr(p, '&');
		size_t seg_len = amp ? (size_t)(amp - p) : strlen(p);
		const char *eq = memchr(p, '=', seg_len);
		size_t k_len = eq ? (size_t)(eq - p) : seg_len;
		char k_dec[48];
		v2_url_decode(p, k_len, k_dec, sizeof(k_dec));
		if (strcmp(k_dec, key) == 0) {
			if (eq)
				v2_url_decode(eq + 1, seg_len - k_len - 1, out, out_sz);
			return;
		}
		if (!amp)
			break;
		p = amp + 1;
	}
}

static int is_safe_http_url(const char *s)
{
	const unsigned char *p = (const unsigned char *)s;
	if (!s || (strncasecmp(s, "https://", 8) != 0 && strncasecmp(s, "http://", 7) != 0))
		return 0;
	while (*p) {
		unsigned char c = *p++;
		if (!(isalnum(c) || c == ':' || c == '/' || c == '.' || c == '-' || c == '_' ||
		      c == '?' || c == '&' || c == '=' || c == '%'))
			return 0;
	}
	return 1;
}

/* ---------------- GET & POST /api_locate.cgi ---------------- */

void do_concard_locate_post(const char *url, FILE *stream, int clen, char *boundary)
{
	char body[512];
	int seconds = 30;
	const char *p;
	char cmd[96];

	(void)url;
	(void)boundary;

	drain_and_read_body(stream, clen, body, sizeof(body));

	if (strstr(body, "action=stop") != NULL || strstr(body, "\"stop\"") != NULL ||
	    strstr(body, "action=off") != NULL) {
		seconds = 0;
	} else if ((p = strstr(body, "seconds=")) != NULL) {
		seconds = atoi(p + 8);
	} else if ((p = strstr(body, "\"seconds\"")) != NULL) {
		p += 9;
		while (*p && (*p == ' ' || *p == '\t' || *p == ':' || *p == '"'))
			p++;
		seconds = atoi(p);
	}

	if (seconds < 0)
		seconds = 0;
	if (seconds > 300)
		seconds = 300;

	if (seconds > 0) {
		unsigned long exp_up = read_proc_uptime_sec() + (unsigned long)seconds;
		FILE *fp = fopen(CONCARD_LOCATE_UNTIL_FILE, "w");
		if (fp) {
			fprintf(fp, "%lu\n", exp_up);
			fclose(fp);
		}
	} else {
		unlink(CONCARD_LOCATE_UNTIL_FILE);
	}

	snprintf(cmd, sizeof(cmd), "/usr/sbin/concard-locate-led %d >/dev/null 2>&1 &", seconds);
	if (system(cmd) < 0) {
		/* ignore */
	}

	g_locate_post_done = 1;
	snprintf(g_locate_post_json, sizeof(g_locate_post_json),
		"{\"ok\":true,\"active\":%s,\"remainingSec\":%d}\n",
		(seconds > 0) ? "true" : "false", seconds);
}

void do_concard_locate_cgi(const char *url, FILE *stream)
{
	unsigned long rem;
	(void)url;

	if (g_locate_post_done) {
		g_locate_post_done = 0;
		fputs(g_locate_post_json, stream);
		fflush(stream);
		return;
	}

	rem = get_locate_remaining_sec();
	fprintf(stream, "{\"ok\":true,\"active\":%s,\"remainingSec\":%lu}\n",
		(rem > 0) ? "true" : "false", rem);
	fflush(stream);
}

/* ---------------- GET & POST /api_ota.cgi ---------------- */

void do_concard_ota_post(const char *url, FILE *stream, int clen, char *boundary)
{
	char body[1024];
	char action[32], repo_url[256], dl_url[384], sha256[80];
	char cmd[640];
	FILE *pp;

	(void)url;
	(void)boundary;

	drain_and_read_body(stream, clen, body, sizeof(body));
	v2_form_param(body, "action", action, sizeof(action));
	v2_form_param(body, "repo_url", repo_url, sizeof(repo_url));
	v2_form_param(body, "download_url", dl_url, sizeof(dl_url));
	v2_form_param(body, "sha256", sha256, sizeof(sha256));

	g_ota_post_done = 1;

	if (strcmp(action, "save_repo") == 0) {
		char e_repo[280];
		if (!is_safe_http_url(repo_url)) {
			snprintf(g_ota_post_json, sizeof(g_ota_post_json),
				"{\"ok\":false,\"error\":\"Invalid GitHub repository URL.\"}\n");
			return;
		}
		nvram_set("cc_ota_repo_url", repo_url);
		nvram_commit_safe();
		v2_json_escape(repo_url, e_repo, sizeof(e_repo));
		snprintf(g_ota_post_json, sizeof(g_ota_post_json),
			"{\"ok\":true,\"repoUrl\":\"%s\",\"message\":\"GitHub OTA repository saved.\"}\n", e_repo);
		return;
	}

	if (strcmp(action, "check") == 0 || action[0] == '\0') {
		const char *use_repo = repo_url[0] ? repo_url : nvram_safe_get("cc_ota_repo_url");
		if (!use_repo || !use_repo[0])
			use_repo = CONCARD_DEF_OTA_REPO;
		if (!is_safe_http_url(use_repo)) {
			snprintf(g_ota_post_json, sizeof(g_ota_post_json),
				"{\"ok\":false,\"error\":\"Invalid GitHub repository URL.\"}\n");
			return;
		}
		if (repo_url[0] && strcmp(repo_url, nvram_safe_get("cc_ota_repo_url")) != 0) {
			nvram_set("cc_ota_repo_url", repo_url);
			nvram_commit_safe();
		}
		snprintf(cmd, sizeof(cmd), "/usr/sbin/concard-ota check '%s' 2>/dev/null", use_repo);
		pp = popen(cmd, "r");
		if (pp) {
			size_t n = fread(g_ota_post_json, 1, sizeof(g_ota_post_json) - 1, pp);
			g_ota_post_json[n] = '\0';
			pclose(pp);
			if (n > 2)
				return;
		}
		snprintf(g_ota_post_json, sizeof(g_ota_post_json),
			"{\"ok\":false,\"apOffline\":true,\"error\":\"OTA check helper unavailable.\"}\n");
		return;
	}

	if (strcmp(action, "upgrade") == 0) {
		size_t si;
		int sha_ok = 1;
		if (!is_safe_http_url(dl_url)) {
			snprintf(g_ota_post_json, sizeof(g_ota_post_json),
				"{\"ok\":false,\"error\":\"Invalid firmware download URL.\"}\n");
			return;
		}
		for (si = 0; sha256[si]; si++) {
			if (!isxdigit((unsigned char)sha256[si])) {
				sha_ok = 0;
				break;
			}
		}
		if (!sha_ok)
			sha256[0] = '\0';
		snprintf(cmd, sizeof(cmd), "/usr/sbin/concard-ota upgrade '%s' '%s' 2>/dev/null", dl_url, sha256);
		pp = popen(cmd, "r");
		if (pp) {
			size_t n = fread(g_ota_post_json, 1, sizeof(g_ota_post_json) - 1, pp);
			g_ota_post_json[n] = '\0';
			pclose(pp);
			if (n > 2)
				return;
		}
		snprintf(g_ota_post_json, sizeof(g_ota_post_json),
			"{\"ok\":false,\"apOffline\":true,\"error\":\"Direct AP download failed.\"}\n");
		return;
	}

	snprintf(g_ota_post_json, sizeof(g_ota_post_json),
		"{\"ok\":false,\"error\":\"Unsupported OTA action.\"}\n");
}

void do_concard_ota_cgi(const char *url, FILE *stream)
{
	FILE *fp;
	const char *ota_u;
	char e_ota[280];

	(void)url;

	if (g_ota_post_done) {
		g_ota_post_done = 0;
		fputs(g_ota_post_json, stream);
		fflush(stream);
		return;
	}

	fp = fopen(CONCARD_OTA_CHECK_FILE, "r");
	if (fp) {
		char buf[1536];
		size_t n = fread(buf, 1, sizeof(buf) - 1, fp);
		fclose(fp);
		if (n > 2) {
			buf[n] = '\0';
			fputs(buf, stream);
			fflush(stream);
			return;
		}
	}

	ota_u = nvram_safe_get("cc_ota_repo_url");
	if (!ota_u || !ota_u[0])
		ota_u = CONCARD_DEF_OTA_REPO;
	v2_json_escape(ota_u, e_ota, sizeof(e_ota));

	fprintf(stream,
		"{\"ok\":true,\"repoUrl\":\"%s\",\"currentVersion\":\"%s\",\"currentBuild\":\"%s\",\"hasUpdate\":false}\n",
		e_ota, CONCARD_VERSION_STR, CONCARD_BUILD_CODE);
	fflush(stream);
}
