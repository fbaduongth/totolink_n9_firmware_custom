/*
 * Concard N9 v2.1 Zero-CPU DHCP Fingerprint & Instant Hostname Daemon (concard-dhcp-fp)
 *
 * Uses a 12-instruction Kernel Classic BPF filter (SO_ATTACH_FILTER) on PF_PACKET
 * so 99.9999% of network packets are filtered in-kernel in <50ns with 0 context switches.
 * Userspace wakes ONLY on client DHCP DISCOVER / REQUEST (UDP 68 -> 67) to extract:
 *   - Option 12: Hostname
 *   - Option 50 / ciaddr: Requested IPv4 Address
 *   - Option 55: Parameter Request List (Darwin iOS / macOS / Android / Windows stack fingerprint)
 *   - Option 60: Vendor Class Identifier (android-dhcp-13/14/15, MSFT 5.0, webOS, Tizen...)
 *
 * Output file: /tmp/.concard_sta_fp  (tab-separated: MAC\tIP\tOS_HINT\tHOSTNAME)
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <ctype.h>
#include <unistd.h>
#include <signal.h>
#include <errno.h>
#include <time.h>
#include <sys/types.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <arpa/inet.h>
#include <linux/if_packet.h>
#include <linux/if_ether.h>
#include <linux/filter.h>

#define PID_FILE	"/var/run/concard_dhcp_fp.pid"
#define OUT_FILE	"/tmp/.concard_sta_fp"
#define OUT_TMP_FILE	"/tmp/.concard_sta_fp.tmp"
#define MAX_FP_ENTRIES	128

typedef struct {
	int used;
	char mac[18];
	char ip[20];
	char os_hint[40];
	char hostname[64];
	unsigned long updated_up;
} fp_entry_t;

static fp_entry_t g_tab[MAX_FP_ENTRIES];
static volatile sig_atomic_t g_stop = 0;

static void on_sig(int sig)
{
	(void)sig;
	g_stop = 1;
}

static unsigned long get_uptime_sec(void)
{
	FILE *fp = fopen("/proc/uptime", "r");
	unsigned long s = 0;
	if (fp) {
		if (fscanf(fp, "%lu", &s) != 1)
			s = 0;
		fclose(fp);
	}
	return s;
}

static int str_icontains(const char *hay, const char *needle)
{
	size_t nlen;
	const char *p;
	if (!hay || !needle || !needle[0])
		return 0;
	nlen = strlen(needle);
	for (p = hay; *p; p++) {
		if (strncasecmp(p, needle, nlen) == 0)
			return 1;
	}
	return 0;
}

static void clean_ascii(const uint8_t *src, size_t len, char *dst, size_t dst_sz)
{
	size_t i, di = 0;
	if (dst_sz == 0)
		return;
	for (i = 0; i < len && di + 1 < dst_sz; i++) {
		uint8_t c = src[i];
		if (c == 0)
			break;
		if (c >= 0x20 && c < 0x7F && c != '\t' && c != '\r' && c != '\n')
			dst[di++] = (char)c;
	}
	while (di > 0 && dst[di - 1] == ' ')
		di--;
	dst[di] = '\0';
}

static void classify_dhcp(const uint8_t *prl, size_t prl_len, const char *vci, const char *host, char *out_os, size_t out_sz)
{
	int has_26 = 0, has_28 = 0, has_43 = 0, has_44 = 0, has_46 = 0;
	int has_95 = 0, has_108 = 0, has_119 = 0, has_121 = 0, has_252 = 0;
	size_t i;

	out_os[0] = '\0';

	for (i = 0; i < prl_len; i++) {
		switch (prl[i]) {
		case 26:  has_26 = 1; break;
		case 28:  has_28 = 1; break;
		case 43:  has_43 = 1; break;
		case 44:  has_44 = 1; break;
		case 46:  has_46 = 1; break;
		case 95:  has_95 = 1; break;
		case 108: has_108 = 1; break;
		case 119: has_119 = 1; break;
		case 121: has_121 = 1; break;
		case 252: has_252 = 1; break;
		default: break;
		}
	}

	if (strncasecmp(vci, "android-dhcp-", 13) == 0) {
		const char *ver = vci + 13;
		char vnum[12] = {0};
		size_t vi = 0;
		while (ver[vi] && (isdigit((unsigned char)ver[vi]) || ver[vi] == '.') && vi + 1 < sizeof(vnum)) {
			vnum[vi] = ver[vi];
			vi++;
		}
		vnum[vi] = '\0';
		if (vnum[0])
			snprintf(out_os, out_sz, "Android %s", vnum);
		else
			snprintf(out_os, out_sz, "Android");
		return;
	}

	if (strncasecmp(vci, "MSFT 5.0", 8) == 0 || strncasecmp(vci, "MSFT ", 5) == 0) {
		snprintf(out_os, out_sz, "Windows PC");
		return;
	}

	if (strncasecmp(vci, "dhcpcd-", 7) == 0) {
		if (str_icontains(vci, "android"))
			snprintf(out_os, out_sz, "Android");
		else
			snprintf(out_os, out_sz, "Linux / ChromeOS");
		return;
	}

	if (str_icontains(vci, "tizen")) {
		snprintf(out_os, out_sz, "Samsung Smart TV");
		return;
	}
	if (str_icontains(vci, "webos") || str_icontains(host, "lgwebostv")) {
		snprintf(out_os, out_sz, "LG webOS TV");
		return;
	}
	if (strncasecmp(vci, "udhcp", 5) == 0) {
		snprintf(out_os, out_sz, "Embedded Linux / IoT");
		return;
	}

	/* Apple Darwin (iOS / iPadOS / macOS) omits Option 60 and sends characteristic Option 55 */
	if (vci[0] == '\0') {
		if ((has_119 && has_252) || (has_121 && has_108 && has_252) || (has_121 && has_119)) {
			if (has_44 || has_46 || has_95 ||
			    str_icontains(host, "macbook") || str_icontains(host, "imac") || str_icontains(host, "mac-")) {
				snprintf(out_os, out_sz, "Apple macOS");
			} else if (str_icontains(host, "ipad")) {
				snprintf(out_os, out_sz, "Apple iPadOS");
			} else if (str_icontains(host, "watch")) {
				snprintf(out_os, out_sz, "Apple watchOS");
			} else {
				snprintf(out_os, out_sz, "Apple iOS");
			}
			return;
		}
		if (has_26 && has_28 && has_43 && !has_252) {
			snprintf(out_os, out_sz, "Android");
			return;
		}
		if (has_44 && has_46 && has_252) {
			snprintf(out_os, out_sz, "Windows PC");
			return;
		}
	}

	if (str_icontains(host, "iphone"))
		snprintf(out_os, out_sz, "Apple iOS");
	else if (str_icontains(host, "ipad"))
		snprintf(out_os, out_sz, "Apple iPadOS");
	else if (str_icontains(host, "macbook") || str_icontains(host, "imac"))
		snprintf(out_os, out_sz, "Apple macOS");
	else if (str_icontains(host, "galaxy") || str_icontains(host, "sm-"))
		snprintf(out_os, out_sz, "Samsung Android");
	else if (strncasecmp(host, "desktop-", 8) == 0 || strncasecmp(host, "laptop-", 7) == 0)
		snprintf(out_os, out_sz, "Windows PC");
}

static void flush_table_to_tmpfs(void)
{
	FILE *fp = fopen(OUT_TMP_FILE, "w");
	int i;
	if (!fp)
		return;
	for (i = 0; i < MAX_FP_ENTRIES; i++) {
		if (!g_tab[i].used || !g_tab[i].mac[0])
			continue;
		fprintf(fp, "%s\t%s\t%s\t%s\n",
			g_tab[i].mac,
			g_tab[i].ip,
			g_tab[i].os_hint,
			g_tab[i].hostname);
	}
	fflush(fp);
	fclose(fp);
	rename(OUT_TMP_FILE, OUT_FILE);
}

static void record_fp(const char *mac, const char *ip, const char *os_hint, const char *host)
{
	int i, slot = -1, oldest_idx = 0;
	unsigned long now_up = get_uptime_sec();
	unsigned long oldest_up = ~0UL;
	int changed = 0;

	for (i = 0; i < MAX_FP_ENTRIES; i++) {
		if (g_tab[i].used && strcmp(g_tab[i].mac, mac) == 0) {
			slot = i;
			break;
		}
		if (!g_tab[i].used && slot < 0) {
			slot = i;
		}
		if (g_tab[i].used && g_tab[i].updated_up < oldest_up) {
			oldest_up = g_tab[i].updated_up;
			oldest_idx = i;
		}
	}

	if (slot < 0)
		slot = oldest_idx;

	if (!g_tab[slot].used || strcmp(g_tab[slot].mac, mac) != 0) {
		memset(&g_tab[slot], 0, sizeof(g_tab[slot]));
		g_tab[slot].used = 1;
		snprintf(g_tab[slot].mac, sizeof(g_tab[slot].mac), "%s", mac);
		changed = 1;
	}

	if (ip && ip[0] && strcmp(ip, "0.0.0.0") != 0 && strncmp(ip, "169.254.", 8) != 0) {
		if (strcmp(g_tab[slot].ip, ip) != 0) {
			snprintf(g_tab[slot].ip, sizeof(g_tab[slot].ip), "%s", ip);
			changed = 1;
		}
	}

	if (os_hint && os_hint[0]) {
		if (strcmp(g_tab[slot].os_hint, os_hint) != 0) {
			snprintf(g_tab[slot].os_hint, sizeof(g_tab[slot].os_hint), "%s", os_hint);
			changed = 1;
		}
	}

	if (host && host[0] && strcmp(host, "*") != 0 &&
	    strcasecmp(host, "null") != 0 && strcasecmp(host, "unknown") != 0 &&
	    strcasecmp(host, "localhost") != 0) {
		if (strcmp(g_tab[slot].hostname, host) != 0) {
			snprintf(g_tab[slot].hostname, sizeof(g_tab[slot].hostname), "%s", host);
			changed = 1;
		}
	}

	g_tab[slot].updated_up = now_up;
	if (changed)
		flush_table_to_tmpfs();
}

static void handle_packet(const uint8_t *pkt, size_t len)
{
	size_t ip_hlen, udp_off, bootp_off, opt_off;
	uint16_t sport, dport;
	const uint8_t *bootp;
	char mac_s[18], ip_s[20] = {0}, host_s[64] = {0}, vci_s[64] = {0}, os_s[40] = {0};
	uint8_t prl[64];
	size_t prl_len = 0;

	if (len < 20 + 8 + 240)
		return;
	if ((pkt[0] >> 4) != 4 || pkt[9] != 17)
		return;

	ip_hlen = (size_t)(pkt[0] & 0x0F) * 4;
	if (ip_hlen < 20 || len < ip_hlen + 8 + 240)
		return;

	udp_off = ip_hlen;
	sport = (uint16_t)((pkt[udp_off] << 8) | pkt[udp_off + 1]);
	dport = (uint16_t)((pkt[udp_off + 2] << 8) | pkt[udp_off + 3]);
	if (sport != 68 || dport != 67)
		return;

	bootp_off = udp_off + 8;
	bootp = pkt + bootp_off;
	if (bootp[0] != 1 || bootp[2] != 6)
		return;

	/* Verify DHCP Magic Cookie 0x63825363 at BOOTP offset 236 */
	if (bootp[236] != 0x63 || bootp[237] != 0x82 || bootp[238] != 0x53 || bootp[239] != 0x63)
		return;

	/* Ignore broadcast/zero chaddr */
	if ((bootp[28] | bootp[29] | bootp[30] | bootp[31] | bootp[32] | bootp[33]) == 0)
		return;

	snprintf(mac_s, sizeof(mac_s), "%02X:%02X:%02X:%02X:%02X:%02X",
		bootp[28], bootp[29], bootp[30], bootp[31], bootp[32], bootp[33]);

	/* Check ciaddr first */
	if (bootp[12] != 0) {
		snprintf(ip_s, sizeof(ip_s), "%u.%u.%u.%u",
			bootp[12], bootp[13], bootp[14], bootp[15]);
	}

	opt_off = bootp_off + 240;
	while (opt_off < len) {
		uint8_t code = pkt[opt_off++];
		uint8_t olen;
		if (code == 0xFF)
			break;
		if (code == 0x00)
			continue;
		if (opt_off >= len)
			break;
		olen = pkt[opt_off++];
		if (opt_off + olen > len)
			break;

		if (code == 12 && olen > 0) {
			clean_ascii(pkt + opt_off, olen, host_s, sizeof(host_s));
		} else if (code == 50 && olen == 4 && pkt[opt_off] != 0) {
			snprintf(ip_s, sizeof(ip_s), "%u.%u.%u.%u",
				pkt[opt_off], pkt[opt_off + 1], pkt[opt_off + 2], pkt[opt_off + 3]);
		} else if (code == 55 && olen > 0) {
			prl_len = (olen < sizeof(prl)) ? olen : sizeof(prl);
			memcpy(prl, pkt + opt_off, prl_len);
		} else if (code == 60 && olen > 0) {
			clean_ascii(pkt + opt_off, olen, vci_s, sizeof(vci_s));
		}

		opt_off += olen;
	}

	classify_dhcp(prl, prl_len, vci_s, host_s, os_s, sizeof(os_s));
	if (os_s[0] || host_s[0] || ip_s[0])
		record_fp(mac_s, ip_s, os_s, host_s);
}

int main(void)
{
	/*
	 * 12-instruction Kernel Classic BPF on cooked IPv4 socket (SOCK_DGRAM + ETH_P_IP):
	 * Accepts ONLY IPv4 UDP packets with sport == 68 and dport == 67.
	 */
	static struct sock_filter dhcp_bpf[] = {
		{ 0x30, 0, 0, 0x00000000 }, /* 0:  ldb [0]                  */
		{ 0x54, 0, 0, 0x000000f0 }, /* 1:  and #0xf0                */
		{ 0x15, 0, 7, 0x00000040 }, /* 2:  jeq #0x40, 3, 10(drop)   */
		{ 0x30, 0, 0, 0x00000009 }, /* 3:  ldb [9] (ip->protocol)   */
		{ 0x15, 0, 5, 0x00000011 }, /* 4:  jeq #17(UDP), 5, 10      */
		{ 0xb1, 0, 0, 0x00000000 }, /* 5:  ldxb 4*([0]&0xf)         */
		{ 0x48, 0, 0, 0x00000000 }, /* 6:  ldh [x + 0] (udp.sport)  */
		{ 0x15, 0, 2, 0x00000044 }, /* 7:  jeq #68, 8, 10(drop)     */
		{ 0x48, 0, 0, 0x00000002 }, /* 8:  ldh [x + 2] (udp.dport)  */
		{ 0x15, 1, 0, 0x00000043 }, /* 9:  jeq #67, 11(keep), 10    */
		{ 0x06, 0, 0, 0x00000000 }, /* 10: ret #0 (DROP)            */
		{ 0x06, 0, 0, 0x00000280 }  /* 11: ret #640 (ACCEPT)        */
	};
	struct sock_fprog fprog = {
		.len = (unsigned short)(sizeof(dhcp_bpf) / sizeof(dhcp_bpf[0])),
		.filter = dhcp_bpf
	};
	int sock;
	FILE *pf;
	uint8_t buf[768];

	pf = fopen(PID_FILE, "r");
	if (pf) {
		long old_pid = 0;
		if (fscanf(pf, "%ld", &old_pid) == 1 && old_pid > 1 && old_pid != (long)getpid()) {
			if (kill((pid_t)old_pid, 0) == 0) {
				fclose(pf);
				return 0;
			}
		}
		fclose(pf);
	}

	pf = fopen(PID_FILE, "w");
	if (pf) {
		fprintf(pf, "%ld\n", (long)getpid());
		fclose(pf);
	}

	signal(SIGINT, on_sig);
	signal(SIGTERM, on_sig);
	signal(SIGHUP, SIG_IGN);

	memset(g_tab, 0, sizeof(g_tab));

	sock = socket(PF_PACKET, SOCK_DGRAM, htons(ETH_P_IP));
	if (sock < 0) {
		unlink(PID_FILE);
		return 1;
	}

	if (setsockopt(sock, SOL_SOCKET, SO_ATTACH_FILTER, &fprog, sizeof(fprog)) < 0) {
		close(sock);
		unlink(PID_FILE);
		return 1;
	}

	while (!g_stop) {
		struct sockaddr_ll sll;
		socklen_t slen = sizeof(sll);
		ssize_t n = recvfrom(sock, buf, sizeof(buf), 0, (struct sockaddr *)&sll, &slen);
		if (n < 0) {
			if (errno == EINTR)
				continue;
			sleep(1);
			continue;
		}
		/* Ignore outgoing packets generated by the AP itself (PACKET_OUTGOING == 4) */
		if (sll.sll_pkttype == PACKET_OUTGOING)
			continue;
		handle_packet(buf, (size_t)n);
	}

	close(sock);
	unlink(PID_FILE);
	return 0;
}
