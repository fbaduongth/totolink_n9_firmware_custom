/*
 * ConCard N9v1 Multi-User Session & Role-Based Authentication Engine for Padavan httpd
 * Supports:
 *   - Cookie-based Web UI sessions (padavan_sid, 12h sliding TTL, multi-client concurrent)
 *   - Stateless HTTP Basic Auth compatibility for ConCard AP Monitor telemetry/provisioning
 *   - Persistent multi-user store (/etc/storage/concard_users.conf) with salted SHA-256
 *   - Automatic bidirectional sync with primary NVRAM http_username / http_passwd & SSH passwd
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <ctype.h>
#include <unistd.h>
#include <fcntl.h>
#include <time.h>
#include <sys/stat.h>
#include <sys/socket.h>
#include <sys/ioctl.h>
#include <net/if.h>
#include <arpa/inet.h>

#include "httpd.h"
#include "concard_auth.h"

#define CONCARD_USERS_PATH    "/etc/storage/concard_users.conf"
#define CONCARD_USERS_TMP     "/tmp/.concard_users.tmp"
#define MAX_WEB_USERS         16
#define MAX_WEB_SESSIONS      32
#define SESSION_TTL_SECONDS   43200UL

typedef struct {
	char username[33];
	char role[12];
	char salt[17];
	char pass_hash[65];
} concard_user_t;

typedef struct {
	int active;
	char token[33];
	char username[33];
	unsigned long last_seen;
} concard_session_t;

static concard_user_t g_users[MAX_WEB_USERS];
static int g_user_count = 0;
static int g_initialized = 0;

static concard_session_t g_sessions[MAX_WEB_SESSIONS];

static char g_last_nvram_user[64] = {0};
static char g_last_nvram_pass[128] = {0};

static char g_req_cookie[512] = {0};
static char g_req_user[33] = {0};
static char g_req_role[12] = {0};
static int g_req_authenticated = 0;
static char g_extra_hdr[320] = {0};
static char g_json_resp[4096] = {0};
static int g_has_post_resp = 0;
static char g_post_action[32] = {0};

/* ---------------- Self-contained FIPS 180-4 SHA-256 ---------------- */

typedef struct {
	uint32_t state[8];
	uint64_t bitcount;
	uint8_t buffer[64];
} sha256_ctx_t;

static const uint32_t k256[64] = {
	0x428a2f98UL, 0x71374491UL, 0xb5c0fbcfUL, 0xe9b5dba5UL,
	0x3956c25bUL, 0x59f111f1UL, 0x923f82a4UL, 0xab1c5ed5UL,
	0xd807aa98UL, 0x12835b01UL, 0x243185beUL, 0x550c7dc3UL,
	0x72be5d74UL, 0x80deb1feUL, 0x9bdc06a7UL, 0xc19bf174UL,
	0xe49b69c1UL, 0xefbe4786UL, 0x0fc19dc6UL, 0x240ca1ccUL,
	0x2de92c6fUL, 0x4a7484aaUL, 0x5cb0a9dcUL, 0x76f988daUL,
	0x983e5152UL, 0xa831c66dUL, 0xb00327c8UL, 0xbf597fc7UL,
	0xc6e00bf3UL, 0xd5a79147UL, 0x06ca6351UL, 0x14292967UL,
	0x27b70a85UL, 0x2e1b2138UL, 0x4d2c6dfcUL, 0x53380d13UL,
	0x650a7354UL, 0x766a0abbUL, 0x81c2c92eUL, 0x92722c85UL,
	0xa2bfe8a1UL, 0xa81a664bUL, 0xc24b8b70UL, 0xc76c51a3UL,
	0xd192e819UL, 0xd6990624UL, 0xf40e3585UL, 0x106aa070UL,
	0x19a4c116UL, 0x1e376c08UL, 0x2748774cUL, 0x34b0bcb5UL,
	0x391c0cb3UL, 0x4ed8aa4aUL, 0x5b9cca4fUL, 0x682e6ff3UL,
	0x748f82eeUL, 0x78a5636fUL, 0x84c87814UL, 0x8cc70208UL,
	0x90befffaUL, 0xa4506cebUL, 0xbef9a3f7UL, 0xc67178f2UL
};

#define ROR32(x, n) (((x) >> (n)) | ((x) << (32 - (n))))

static void sha256_transform(sha256_ctx_t *ctx, const uint8_t data[64])
{
	uint32_t w[64], a, b, c, d, e, f, g, h, t1, t2;
	int i;

	for (i = 0; i < 16; i++) {
		w[i] = ((uint32_t)data[i * 4] << 24) |
		       ((uint32_t)data[i * 4 + 1] << 16) |
		       ((uint32_t)data[i * 4 + 2] << 8) |
		       ((uint32_t)data[i * 4 + 3]);
	}
	for (i = 16; i < 64; i++) {
		uint32_t s0 = ROR32(w[i - 15], 7) ^ ROR32(w[i - 15], 18) ^ (w[i - 15] >> 3);
		uint32_t s1 = ROR32(w[i - 2], 17) ^ ROR32(w[i - 2], 19) ^ (w[i - 2] >> 10);
		w[i] = w[i - 16] + s0 + w[i - 7] + s1;
	}

	a = ctx->state[0]; b = ctx->state[1]; c = ctx->state[2]; d = ctx->state[3];
	e = ctx->state[4]; f = ctx->state[5]; g = ctx->state[6]; h = ctx->state[7];

	for (i = 0; i < 64; i++) {
		uint32_t S1 = ROR32(e, 6) ^ ROR32(e, 11) ^ ROR32(e, 25);
		uint32_t ch = (e & f) ^ ((~e) & g);
		uint32_t S0 = ROR32(a, 2) ^ ROR32(a, 13) ^ ROR32(a, 22);
		uint32_t maj = (a & b) ^ (a & c) ^ (b & c);
		t1 = h + S1 + ch + k256[i] + w[i];
		t2 = S0 + maj;
		h = g; g = f; f = e; e = d + t1;
		d = c; c = b; b = a; a = t1 + t2;
	}

	ctx->state[0] += a; ctx->state[1] += b; ctx->state[2] += c; ctx->state[3] += d;
	ctx->state[4] += e; ctx->state[5] += f; ctx->state[6] += g; ctx->state[7] += h;
}

static void sha256_init(sha256_ctx_t *ctx)
{
	ctx->state[0] = 0x6a09e667UL; ctx->state[1] = 0xbb67ae85UL;
	ctx->state[2] = 0x3c6ef372UL; ctx->state[3] = 0xa54ff53aUL;
	ctx->state[4] = 0x510e527fUL; ctx->state[5] = 0x9b05688cUL;
	ctx->state[6] = 0x1f83d9abUL; ctx->state[7] = 0x5be0cd19UL;
	ctx->bitcount = 0;
	memset(ctx->buffer, 0, sizeof(ctx->buffer));
}

static void sha256_update(sha256_ctx_t *ctx, const uint8_t *data, size_t len)
{
	size_t idx = (size_t)((ctx->bitcount >> 3) & 0x3F);
	size_t i = 0;
	ctx->bitcount += ((uint64_t)len << 3);

	if (idx) {
		size_t left = 64 - idx;
		if (len < left) {
			memcpy(ctx->buffer + idx, data, len);
			return;
		}
		memcpy(ctx->buffer + idx, data, left);
		sha256_transform(ctx, ctx->buffer);
		i = left;
	}
	for (; i + 64 <= len; i += 64)
		sha256_transform(ctx, data + i);
	if (i < len)
		memcpy(ctx->buffer, data + i, len - i);
}

static void sha256_final_hex(sha256_ctx_t *ctx, char out_hex[65])
{
	static const char hex[] = "0123456789abcdef";
	uint8_t pad[64];
	uint8_t len_be[8];
	uint64_t bits = ctx->bitcount;
	size_t idx = (size_t)((ctx->bitcount >> 3) & 0x3F);
	size_t pad_len = (idx < 56) ? (56 - idx) : (120 - idx);
	int i;

	memset(pad, 0, sizeof(pad));
	pad[0] = 0x80;
	for (i = 0; i < 8; i++)
		len_be[7 - i] = (uint8_t)((bits >> (i * 8)) & 0xFF);

	sha256_update(ctx, pad, pad_len);
	sha256_update(ctx, len_be, 8);

	for (i = 0; i < 8; i++) {
		uint32_t v = ctx->state[i];
		out_hex[i * 8 + 0] = hex[(v >> 28) & 0xF];
		out_hex[i * 8 + 1] = hex[(v >> 24) & 0xF];
		out_hex[i * 8 + 2] = hex[(v >> 20) & 0xF];
		out_hex[i * 8 + 3] = hex[(v >> 16) & 0xF];
		out_hex[i * 8 + 4] = hex[(v >> 12) & 0xF];
		out_hex[i * 8 + 5] = hex[(v >> 8) & 0xF];
		out_hex[i * 8 + 6] = hex[(v >> 4) & 0xF];
		out_hex[i * 8 + 7] = hex[v & 0xF];
	}
	out_hex[64] = '\0';
}

static void hash_salted_password(const char *salt, const char *password, char out_hex[65])
{
	sha256_ctx_t ctx;
	sha256_init(&ctx);
	sha256_update(&ctx, (const uint8_t *)salt, strlen(salt));
	sha256_update(&ctx, (const uint8_t *)":", 1);
	sha256_update(&ctx, (const uint8_t *)password, strlen(password));
	sha256_final_hex(&ctx, out_hex);
}

static int ct_str_eq(const char *a, const char *b)
{
	size_t la = strlen(a), lb = strlen(b), i;
	unsigned char diff = (unsigned char)(la ^ lb);
	size_t n = (la < lb) ? la : lb;
	for (i = 0; i < n; i++)
		diff |= (unsigned char)(a[i] ^ b[i]);
	return (diff == 0 && la == lb) ? 1 : 0;
}

static void gen_random_hex(char *out, size_t nbytes)
{
	static const char hex[] = "0123456789abcdef";
	uint8_t buf[32];
	size_t i;
	int fd;

	if (nbytes > sizeof(buf))
		nbytes = sizeof(buf);

	memset(buf, 0, sizeof(buf));
	fd = open("/dev/urandom", O_RDONLY);
	if (fd >= 0) {
		ssize_t r = read(fd, buf, nbytes);
		(void)r;
		close(fd);
	} else {
		unsigned int seed = (unsigned int)(time(NULL) ^ (uptime() * 2654435761UL) ^ getpid());
		for (i = 0; i < nbytes; i++) {
			seed = seed * 1103515245U + 12345U;
			buf[i] = (uint8_t)((seed >> 16) & 0xFF);
		}
	}

	for (i = 0; i < nbytes; i++) {
		out[i * 2] = hex[(buf[i] >> 4) & 0xF];
		out[i * 2 + 1] = hex[buf[i] & 0xF];
	}
	out[nbytes * 2] = '\0';
}

/* ---------------- Helpers: Escaping, Validation, Form Parsing ---------------- */

static void json_escape(const char *src, char *dst, size_t dst_sz)
{
	size_t di = 0;
	const unsigned char *p = (const unsigned char *)(src ? src : "");

	if (dst_sz == 0)
		return;

	while (*p && di + 2 < dst_sz) {
		unsigned char c = *p++;
		if (c == '"' || c == '\\') {
			if (di + 3 >= dst_sz) break;
			dst[di++] = '\\';
			dst[di++] = (char)c;
		} else if (c == '\n') {
			if (di + 3 >= dst_sz) break;
			dst[di++] = '\\';
			dst[di++] = 'n';
		} else if (c == '\r') {
			if (di + 3 >= dst_sz) break;
			dst[di++] = '\\';
			dst[di++] = 'r';
		} else if (c == '\t') {
			if (di + 3 >= dst_sz) break;
			dst[di++] = '\\';
			dst[di++] = 't';
		} else if (c >= 0x20) {
			dst[di++] = (char)c;
		}
	}
	dst[di] = '\0';
}

static int is_valid_username(const char *u)
{
	size_t len, i;
	if (!u) return 0;
	len = strlen(u);
	if (len < 1 || len > 32) return 0;
	for (i = 0; i < len; i++) {
		unsigned char c = (unsigned char)u[i];
		if (!(isalnum(c) || c == '_' || c == '-' || c == '.'))
			return 0;
	}
	return 1;
}

static int is_valid_password(const char *p)
{
	size_t len, i;
	if (!p) return 0;
	len = strlen(p);
	if (len < 4 || len > 64) return 0;
	for (i = 0; i < len; i++) {
		unsigned char c = (unsigned char)p[i];
		if (c < 0x20 || c == 0x7F)
			return 0;
	}
	return 1;
}

static int is_valid_role(const char *r)
{
	if (!r) return 0;
	return (strcmp(r, CONCARD_ROLE_ADMIN) == 0 || strcmp(r, CONCARD_ROLE_VIEWER) == 0) ? 1 : 0;
}

static const char *get_primary_nvram_user(void)
{
	const char *u = nvram_safe_get("http_username");
	return (u && u[0]) ? u : "admin";
}

static const char *get_primary_nvram_pass(void)
{
	const char *p = nvram_safe_get("http_passwd");
	return (p && p[0]) ? p : "admin";
}

static int find_user_idx(const char *username)
{
	int i;
	if (!username || !username[0]) return -1;
	for (i = 0; i < g_user_count; i++) {
		if (strcmp(g_users[i].username, username) == 0)
			return i;
	}
	return -1;
}

static int count_admins(void)
{
	int i, n = 0;
	for (i = 0; i < g_user_count; i++) {
		if (strcmp(g_users[i].role, CONCARD_ROLE_ADMIN) == 0)
			n++;
	}
	return n;
}

static int save_users_file(int commit_flash)
{
	FILE *fp;
	int i;

	mkdir("/etc/storage", 0755);
	fp = fopen(CONCARD_USERS_TMP, "w");
	if (!fp)
		fp = fopen(CONCARD_USERS_PATH, "w");
	if (!fp)
		return -1;

	for (i = 0; i < g_user_count; i++) {
		fprintf(fp, "%s\t%s\t%s\t%s\n",
			g_users[i].username,
			g_users[i].role,
			g_users[i].salt,
			g_users[i].pass_hash);
	}
	fflush(fp);
	fclose(fp);

	if (access(CONCARD_USERS_TMP, F_OK) == 0) {
		if (rename(CONCARD_USERS_TMP, CONCARD_USERS_PATH) != 0) {
			doSystem("cp -f %s %s && rm -f %s", CONCARD_USERS_TMP, CONCARD_USERS_PATH, CONCARD_USERS_TMP);
		}
	}
	chmod(CONCARD_USERS_PATH, 0600);

	if (commit_flash) {
		doSystem("/sbin/mtd_storage.sh %s >/dev/null 2>&1 &", "save");
	}
	return 0;
}

static void set_user_password(concard_user_t *u, const char *plain_pass)
{
	gen_random_hex(u->salt, 8);
	hash_salted_password(u->salt, plain_pass, u->pass_hash);
}

static void revoke_sessions_for_user(const char *username)
{
	int i;
	for (i = 0; i < MAX_WEB_SESSIONS; i++) {
		if (g_sessions[i].active && strcmp(g_sessions[i].username, username) == 0) {
			memset(&g_sessions[i], 0, sizeof(g_sessions[i]));
		}
	}
}

void concard_auth_init(void)
{
	FILE *fp;
	char line[256];
	const char *prim_u = get_primary_nvram_user();
	const char *prim_p = get_primary_nvram_pass();

	if (g_initialized)
		return;
	g_initialized = 1;
	g_user_count = 0;
	memset(g_users, 0, sizeof(g_users));
	memset(g_sessions, 0, sizeof(g_sessions));

	snprintf(g_last_nvram_user, sizeof(g_last_nvram_user), "%s", prim_u);
	snprintf(g_last_nvram_pass, sizeof(g_last_nvram_pass), "%s", prim_p);

	fp = fopen(CONCARD_USERS_PATH, "r");
	if (fp) {
		while (fgets(line, sizeof(line), fp) && g_user_count < MAX_WEB_USERS) {
			char *u, *r, *s, *h, *nl;
			nl = strpbrk(line, "\r\n");
			if (nl) *nl = '\0';
			if (line[0] == '\0' || line[0] == '#')
				continue;
			u = strtok(line, "\t");
			r = strtok(NULL, "\t");
			s = strtok(NULL, "\t");
			h = strtok(NULL, "\t");
			if (!u || !r || !s || !h)
				continue;
			if (!is_valid_username(u) || !is_valid_role(r) || strlen(s) < 4 || strlen(h) != 64)
				continue;
			if (find_user_idx(u) >= 0)
				continue;
			snprintf(g_users[g_user_count].username, sizeof(g_users[0].username), "%s", u);
			snprintf(g_users[g_user_count].role, sizeof(g_users[0].role), "%s", r);
			snprintf(g_users[g_user_count].salt, sizeof(g_users[0].salt), "%s", s);
			snprintf(g_users[g_user_count].pass_hash, sizeof(g_users[0].pass_hash), "%s", h);
			g_user_count++;
		}
		fclose(fp);
	}

	/* Ensure primary NVRAM admin exists and at least 1 admin account is active */
	if (g_user_count == 0 || count_admins() == 0) {
		int idx = find_user_idx(prim_u);
		if (idx < 0 && g_user_count < MAX_WEB_USERS) {
			idx = g_user_count++;
			snprintf(g_users[idx].username, sizeof(g_users[idx].username), "%s",
				is_valid_username(prim_u) ? prim_u : "admin");
		}
		if (idx >= 0) {
			snprintf(g_users[idx].role, sizeof(g_users[idx].role), "%s", CONCARD_ROLE_ADMIN);
			set_user_password(&g_users[idx], prim_p);
		}
		save_users_file(0);
	} else {
		/* Sync primary NVRAM user password in case NVRAM was updated before reboot */
		int idx = find_user_idx(prim_u);
		if (idx >= 0) {
			char calc[65];
			hash_salted_password(g_users[idx].salt, prim_p, calc);
			if (!ct_str_eq(calc, g_users[idx].pass_hash)) {
				set_user_password(&g_users[idx], prim_p);
				snprintf(g_users[idx].role, sizeof(g_users[idx].role), "%s", CONCARD_ROLE_ADMIN);
				save_users_file(0);
			}
		} else if (g_user_count < MAX_WEB_USERS && is_valid_username(prim_u)) {
			idx = g_user_count++;
			snprintf(g_users[idx].username, sizeof(g_users[idx].username), "%s", prim_u);
			snprintf(g_users[idx].role, sizeof(g_users[idx].role), "%s", CONCARD_ROLE_ADMIN);
			set_user_password(&g_users[idx], prim_p);
			save_users_file(0);
		}
	}
}

void concard_auth_sync_nvram(void)
{
	const char *prim_u, *prim_p;
	int idx;

	if (!g_initialized)
		concard_auth_init();

	prim_u = get_primary_nvram_user();
	prim_p = get_primary_nvram_pass();

	if (strcmp(prim_u, g_last_nvram_user) == 0 && strcmp(prim_p, g_last_nvram_pass) == 0)
		return;

	/* Primary NVRAM credentials changed (e.g. via start_apply.htm from ConCard AP Monitor) */
	idx = find_user_idx(g_last_nvram_user);
	if (idx >= 0 && strcmp(g_last_nvram_user, prim_u) != 0 && find_user_idx(prim_u) < 0 && is_valid_username(prim_u)) {
		revoke_sessions_for_user(g_users[idx].username);
		snprintf(g_users[idx].username, sizeof(g_users[idx].username), "%s", prim_u);
	} else {
		idx = find_user_idx(prim_u);
		if (idx < 0 && g_user_count < MAX_WEB_USERS && is_valid_username(prim_u)) {
			idx = g_user_count++;
			snprintf(g_users[idx].username, sizeof(g_users[idx].username), "%s", prim_u);
		}
	}

	if (idx >= 0) {
		snprintf(g_users[idx].role, sizeof(g_users[idx].role), "%s", CONCARD_ROLE_ADMIN);
		set_user_password(&g_users[idx], prim_p);
		save_users_file(1);
	}

	snprintf(g_last_nvram_user, sizeof(g_last_nvram_user), "%s", prim_u);
	snprintf(g_last_nvram_pass, sizeof(g_last_nvram_pass), "%s", prim_p);
}

void concard_auth_begin_request(void)
{
	if (!g_initialized)
		concard_auth_init();
	g_req_cookie[0] = '\0';
	g_req_user[0] = '\0';
	g_req_role[0] = '\0';
	g_req_authenticated = 0;
	g_extra_hdr[0] = '\0';
	g_json_resp[0] = '\0';
	g_has_post_resp = 0;
	g_post_action[0] = '\0';
}

const char *concard_auth_get_extra_header(void)
{
	return g_extra_hdr[0] ? g_extra_hdr : NULL;
}

static int extract_cookie_token(const char *cookie_hdr, char out_tok[33])
{
	const char *p;
	size_t i = 0;
	out_tok[0] = '\0';
	if (!cookie_hdr || !cookie_hdr[0])
		return 0;

	p = strstr(cookie_hdr, "padavan_sid=");
	while (p) {
		if (p == cookie_hdr || p[-1] == ' ' || p[-1] == ';' || p[-1] == '\t') {
			p += 12;
			while (*p && *p != ';' && *p != ' ' && *p != '\r' && *p != '\n' && i < 32) {
				if (!isxdigit((unsigned char)*p))
					return 0;
				out_tok[i++] = *p++;
			}
			out_tok[i] = '\0';
			return (i == 32) ? 1 : 0;
		}
		p = strstr(p + 12, "padavan_sid=");
	}
	return 0;
}

static int verify_user_password(const char *username, const char *password, char *out_role, size_t role_sz)
{
	int idx;
	char calc[65];

	concard_auth_sync_nvram();
	idx = find_user_idx(username);
	if (idx < 0 || !password)
		return 0;

	hash_salted_password(g_users[idx].salt, password, calc);
	if (!ct_str_eq(calc, g_users[idx].pass_hash))
		return 0;

	if (out_role && role_sz > 0)
		snprintf(out_role, role_sz, "%s", g_users[idx].role);
	return 1;
}

int concard_auth_check_request(
	const char *cookie_hdr,
	const char *authorization_hdr,
	char *out_user,
	size_t out_user_sz,
	char *out_role,
	size_t out_role_sz)
{
	char tok[33];
	unsigned long now = (unsigned long)uptime();
	int i;

	concard_auth_sync_nvram();

	if (cookie_hdr && cookie_hdr[0]) {
		snprintf(g_req_cookie, sizeof(g_req_cookie), "%s", cookie_hdr);
	}

	/* 1. Check Session Cookie (padavan_sid) */
	if (extract_cookie_token(cookie_hdr, tok)) {
		for (i = 0; i < MAX_WEB_SESSIONS; i++) {
			if (!g_sessions[i].active)
				continue;
			if ((unsigned long)(now - g_sessions[i].last_seen) > SESSION_TTL_SECONDS) {
				memset(&g_sessions[i], 0, sizeof(g_sessions[i]));
				continue;
			}
			if (ct_str_eq(g_sessions[i].token, tok)) {
				int uidx = find_user_idx(g_sessions[i].username);
				if (uidx < 0) {
					memset(&g_sessions[i], 0, sizeof(g_sessions[i]));
					break;
				}
				g_sessions[i].last_seen = now;
				snprintf(g_req_user, sizeof(g_req_user), "%s", g_users[uidx].username);
				snprintf(g_req_role, sizeof(g_req_role), "%s", g_users[uidx].role);
				g_req_authenticated = 1;
				if (out_user && out_user_sz > 0)
					snprintf(out_user, out_user_sz, "%s", g_req_user);
				if (out_role && out_role_sz > 0)
					snprintf(out_role, out_role_sz, "%s", g_req_role);
				return 1;
			}
		}
	}

	/* 2. Check Stateless HTTP Basic Auth (ConCard AP Monitor & CLI tools) */
	if (authorization_hdr && strncasecmp(authorization_hdr, "Basic ", 6) == 0) {
		unsigned char decoded[256];
		int dlen;
		char *colon;
		const char *b64 = authorization_hdr + 6;
		while (*b64 == ' ' || *b64 == '\t') b64++;

		memset(decoded, 0, sizeof(decoded));
		dlen = b64_decode(b64, decoded, sizeof(decoded) - 1);
		if (dlen > 0) {
			decoded[dlen] = '\0';
			colon = strchr((char *)decoded, ':');
			if (colon) {
				char role_buf[12] = {0};
				*colon = '\0';
				if (verify_user_password((const char *)decoded, colon + 1, role_buf, sizeof(role_buf))) {
					snprintf(g_req_user, sizeof(g_req_user), "%s", (const char *)decoded);
					snprintf(g_req_role, sizeof(g_req_role), "%s", role_buf);
					g_req_authenticated = 2;
					if (out_user && out_user_sz > 0)
						snprintf(out_user, out_user_sz, "%s", g_req_user);
					if (out_role && out_role_sz > 0)
						snprintf(out_role, out_role_sz, "%s", g_req_role);
					return 2;
				}
			}
		}
	}

	if (out_user && out_user_sz > 0) out_user[0] = '\0';
	if (out_role && out_role_sz > 0) out_role[0] = '\0';
	return 0;
}

int concard_auth_is_viewer_forbidden(const char *file, int method_id)
{
	if (!file)
		return 1;

	/* Allow read-only viewer on auth, netdev traffic stats, and self password change */
	if (strncmp(file, "api_auth.cgi", 12) == 0)
		return 0;
	if (strncmp(file, "update.cgi", 10) == 0)
		return 0;
	if (strncmp(file, "api_users.cgi", 13) == 0) {
		if (method_id != 2) /* GET allowed (returns own user info only) */
			return 0;
		/* POST to api_users.cgi: handled inside do_concard_users_post (allows self_passwd only) */
		return 0;
	}

	/* Block any other POST for viewer */
	if (method_id == 2)
		return 1;

	/* Block config/storage downloads & mutating pages via GET */
	if (strncmp(file, "Settings_", 9) == 0 ||
	    strncmp(file, "Storage_", 8) == 0 ||
	    strncmp(file, "start_apply.htm", 15) == 0 ||
	    strncmp(file, "apply.cgi", 9) == 0 ||
	    strncmp(file, "upgrade.cgi", 11) == 0 ||
	    strncmp(file, "restore_", 8) == 0)
		return 1;

	return 0;
}

/* ---------------- URL-encoded POST Body Reader ---------------- */

static void read_post_urlencoded(FILE *stream, int clen, char *out_buf, size_t out_sz)
{
	size_t to_read, got = 0;
	if (out_sz == 0)
		return;
	out_buf[0] = '\0';
	if (clen <= 0)
		return;

	to_read = (size_t)clen;
	if (to_read >= out_sz)
		to_read = out_sz - 1;

	while (got < to_read) {
		size_t r = fread(out_buf + got, 1, to_read - got, stream);
		if (r == 0)
			break;
		got += r;
	}
	out_buf[got] = '\0';

	/* Drain any remaining bytes beyond out_sz */
	if ((size_t)clen > got) {
		size_t rem = (size_t)clen - got;
		char drain[256];
		while (rem > 0) {
			size_t chunk = (rem > sizeof(drain)) ? sizeof(drain) : rem;
			if (fread(drain, 1, chunk, stream) == 0)
				break;
			rem -= chunk;
		}
	}
}

static int hex_nibble(char c)
{
	if (c >= '0' && c <= '9') return c - '0';
	if (c >= 'a' && c <= 'f') return c - 'a' + 10;
	if (c >= 'A' && c <= 'F') return c - 'A' + 10;
	return -1;
}

static void url_decode_slice(const char *src, size_t len, char *dst, size_t dst_sz)
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
			int hi = hex_nibble(src[si + 1]);
			int lo = hex_nibble(src[si + 2]);
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

static void form_get_param(const char *body, const char *key, const char *def_val, char *out, size_t out_sz)
{
	const char *p = body ? body : "";
	if (out_sz == 0)
		return;
	while (*p) {
		const char *amp = strchr(p, '&');
		size_t seg_len = amp ? (size_t)(amp - p) : strlen(p);
		const char *eq = memchr(p, '=', seg_len);
		size_t k_len = eq ? (size_t)(eq - p) : seg_len;
		char k_dec[64];
		url_decode_slice(p, k_len, k_dec, sizeof(k_dec));
		if (strcmp(k_dec, key) == 0) {
			if (eq) {
				size_t v_len = seg_len - k_len - 1;
				url_decode_slice(eq + 1, v_len, out, out_sz);
			} else {
				out[0] = '\0';
			}
			return;
		}
		if (!amp)
			break;
		p = amp + 1;
	}
	snprintf(out, out_sz, "%s", def_val ? def_val : "");
}

static void create_session(const char *username, char out_tok[33])
{
	unsigned long now = (unsigned long)uptime();
	int i, slot = -1;
	unsigned long oldest_time = ~0UL;

	for (i = 0; i < MAX_WEB_SESSIONS; i++) {
		if (!g_sessions[i].active || (unsigned long)(now - g_sessions[i].last_seen) > SESSION_TTL_SECONDS) {
			slot = i;
			break;
		}
		if (g_sessions[i].last_seen < oldest_time) {
			oldest_time = g_sessions[i].last_seen;
			slot = i;
		}
	}
	if (slot < 0)
		slot = 0;

	gen_random_hex(out_tok, 16);
	g_sessions[slot].active = 1;
	snprintf(g_sessions[slot].token, sizeof(g_sessions[slot].token), "%s", out_tok);
	snprintf(g_sessions[slot].username, sizeof(g_sessions[slot].username), "%s", username);
	g_sessions[slot].last_seen = now;
}

/* ---------------- /api_auth.cgi ---------------- */

void do_concard_auth_post(const char *url, FILE *stream, int clen, char *boundary)
{
	char body[1024];
	char action[32], username[64], password[128];
	(void)url;
	(void)boundary;

	read_post_urlencoded(stream, clen, body, sizeof(body));
	form_get_param(body, "action", "login", action, sizeof(action));
	form_get_param(body, "username", "", username, sizeof(username));
	form_get_param(body, "password", "", password, sizeof(password));
	g_has_post_resp = 1;

	if (strcmp(action, "logout") == 0) {
		char tok[33];
		if (extract_cookie_token(g_req_cookie, tok)) {
			int i;
			for (i = 0; i < MAX_WEB_SESSIONS; i++) {
				if (g_sessions[i].active && ct_str_eq(g_sessions[i].token, tok))
					memset(&g_sessions[i], 0, sizeof(g_sessions[i]));
			}
		}
		snprintf(g_extra_hdr, sizeof(g_extra_hdr),
			"Cache-Control: no-store, no-cache, must-revalidate\r\n"
			"Pragma: no-cache\r\n"
			"Set-Cookie: padavan_sid=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax");
		snprintf(g_json_resp, sizeof(g_json_resp),
			"{\"ok\":true,\"authenticated\":false}");
		return;
	}

	if (strcmp(action, "login") == 0) {
		char role[12] = {0};
		if (verify_user_password(username, password, role, sizeof(role))) {
			char tok[33], eu[80], er[32];
			create_session(username, tok);
			json_escape(username, eu, sizeof(eu));
			json_escape(role, er, sizeof(er));
			snprintf(g_extra_hdr, sizeof(g_extra_hdr),
				"Cache-Control: no-store, no-cache, must-revalidate\r\n"
				"Pragma: no-cache\r\n"
				"Set-Cookie: padavan_sid=%s; Path=/; Max-Age=%lu; HttpOnly; SameSite=Lax",
				tok, SESSION_TTL_SECONDS);
			snprintf(g_json_resp, sizeof(g_json_resp),
				"{\"ok\":true,\"authenticated\":true,\"user\":\"%s\",\"role\":\"%s\"}",
				eu, er);
		} else {
			usleep(150000);
			snprintf(g_extra_hdr, sizeof(g_extra_hdr),
				"Cache-Control: no-store, no-cache, must-revalidate\r\nPragma: no-cache");
			snprintf(g_json_resp, sizeof(g_json_resp),
				"{\"ok\":false,\"authenticated\":false,\"error\":\"Invalid username or password.\"}");
		}
		return;
	}

	snprintf(g_json_resp, sizeof(g_json_resp),
		"{\"ok\":false,\"error\":\"Unsupported auth action.\"}");
}

static void get_live_br_ip(char *out, size_t out_sz)
{
	int fd;
	struct ifreq ifr;
	const char *nv_ip;

	fd = socket(AF_INET, SOCK_DGRAM, 0);
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

void do_concard_auth_cgi(const char *url, FILE *stream)
{
	char live_ip[32], eh[96], eip[64], efw[96], eu[80], er[32];
	const char *host, *fw;
	int cookie_authed;
	(void)url;

	if (g_has_post_resp) {
		fputs(g_json_resp, stream);
		fflush(stream);
		return;
	}

	cookie_authed = (g_req_authenticated == 1);
	host = nvram_safe_get("computer_name");
	if (!host || !host[0])
		host = nvram_safe_get("productid");
	get_live_br_ip(live_ip, sizeof(live_ip));
	fw = nvram_safe_get("firmver_sub");

	json_escape(host, eh, sizeof(eh));
	json_escape(live_ip, eip, sizeof(eip));
	json_escape(fw, efw, sizeof(efw));
	json_escape(g_req_user, eu, sizeof(eu));
	json_escape(g_req_role, er, sizeof(er));

	fprintf(stream,
		"{\"ok\":true,\"authenticated\":%s,\"user\":\"%s\",\"role\":\"%s\","
		"\"hostname\":\"%s\",\"ip\":\"%s\",\"firmver\":\"%s\"}",
		cookie_authed ? "true" : "false",
		cookie_authed ? eu : "",
		cookie_authed ? er : "",
		eh, eip, efw);
	fflush(stream);
}

/* ---------------- /api_users.cgi ---------------- */

static void build_users_list_json(char *out, size_t out_sz, const char *msg)
{
	size_t off = 0;
	char ecu[80], ecr[32], epu[80], emsg[160];
	const char *prim_u = get_primary_nvram_user();
	int i, is_admin = (strcmp(g_req_role, CONCARD_ROLE_ADMIN) == 0);
	int first = 1;

	json_escape(g_req_user, ecu, sizeof(ecu));
	json_escape(g_req_role, ecr, sizeof(ecr));
	json_escape(prim_u, epu, sizeof(epu));
	json_escape(msg ? msg : "", emsg, sizeof(emsg));

	off += (size_t)snprintf(out + off, out_sz - off,
		"{\"ok\":true,\"current_user\":\"%s\",\"current_role\":\"%s\","
		"\"primary_user\":\"%s\",\"message\":\"%s\",\"users\":[",
		ecu, ecr, epu, emsg);

	for (i = 0; i < g_user_count && off + 160 < out_sz; i++) {
		char eu[80], er[32];
		int is_prim;
		if (!is_admin && strcmp(g_users[i].username, g_req_user) != 0)
			continue;
		json_escape(g_users[i].username, eu, sizeof(eu));
		json_escape(g_users[i].role, er, sizeof(er));
		is_prim = (strcmp(g_users[i].username, prim_u) == 0);
		off += (size_t)snprintf(out + off, out_sz - off,
			"%s{\"username\":\"%s\",\"role\":\"%s\",\"is_primary\":%s}",
			first ? "" : ",",
			eu, er,
			is_prim ? "true" : "false");
		first = 0;
	}

	snprintf(out + off, out_sz - off, "]}");
}

static void sync_primary_nvram_password_if_needed(const char *username, const char *new_pass)
{
	const char *prim_u = get_primary_nvram_user();
	if (strcmp(username, prim_u) == 0 && new_pass && new_pass[0]) {
		nvram_set("http_passwd", (char *)new_pass);
		nvram_commit_safe();
		change_passwd_unix((char *)prim_u, (char *)new_pass);
		snprintf(g_last_nvram_user, sizeof(g_last_nvram_user), "%s", prim_u);
		snprintf(g_last_nvram_pass, sizeof(g_last_nvram_pass), "%s", new_pass);
	}
}

void do_concard_users_post(const char *url, FILE *stream, int clen, char *boundary)
{
	char body[1024];
	char action[32], username[64], password[128], old_password[128], role[24];
	const char *prim_u = get_primary_nvram_user();
	int idx;
	(void)url;
	(void)boundary;

	read_post_urlencoded(stream, clen, body, sizeof(body));
	form_get_param(body, "action", "", action, sizeof(action));
	form_get_param(body, "username", "", username, sizeof(username));
	form_get_param(body, "password", "", password, sizeof(password));
	form_get_param(body, "old_password", "", old_password, sizeof(old_password));
	form_get_param(body, "role", CONCARD_ROLE_VIEWER, role, sizeof(role));

	snprintf(g_post_action, sizeof(g_post_action), "%s", action);
	g_has_post_resp = 1;

	if (!g_req_authenticated || !g_req_user[0]) {
		snprintf(g_json_resp, sizeof(g_json_resp),
			"{\"ok\":false,\"auth_required\":true,\"error\":\"Authentication required.\"}");
		return;
	}

	/* Self password change (allowed for both admin and viewer) */
	if (strcmp(action, "self_passwd") == 0) {
		char dummy_role[12];
		idx = find_user_idx(g_req_user);
		if (idx < 0 || !verify_user_password(g_req_user, old_password, dummy_role, sizeof(dummy_role))) {
			snprintf(g_json_resp, sizeof(g_json_resp),
				"{\"ok\":false,\"error\":\"Current password is incorrect.\"}");
			return;
		}
		if (!is_valid_password(password)) {
			snprintf(g_json_resp, sizeof(g_json_resp),
				"{\"ok\":false,\"error\":\"New password must be 4 to 64 printable characters.\"}");
			return;
		}
		set_user_password(&g_users[idx], password);
		sync_primary_nvram_password_if_needed(g_users[idx].username, password);
		save_users_file(1);
		build_users_list_json(g_json_resp, sizeof(g_json_resp), "Password updated successfully.");
		return;
	}

	/* All other user management actions require Administrator role */
	if (strcmp(g_req_role, CONCARD_ROLE_ADMIN) != 0) {
		snprintf(g_json_resp, sizeof(g_json_resp),
			"{\"ok\":false,\"error\":\"Administrator privileges required to manage users.\"}");
		return;
	}

	if (strcmp(action, "add") == 0) {
		if (!is_valid_username(username)) {
			snprintf(g_json_resp, sizeof(g_json_resp),
				"{\"ok\":false,\"error\":\"Invalid username (1-32 chars: letters, digits, . _ -).\"}");
			return;
		}
		if (find_user_idx(username) >= 0) {
			snprintf(g_json_resp, sizeof(g_json_resp),
				"{\"ok\":false,\"error\":\"Username already exists.\"}");
			return;
		}
		if (g_user_count >= MAX_WEB_USERS) {
			snprintf(g_json_resp, sizeof(g_json_resp),
				"{\"ok\":false,\"error\":\"Maximum number of user accounts (16) reached.\"}");
			return;
		}
		if (!is_valid_password(password)) {
			snprintf(g_json_resp, sizeof(g_json_resp),
				"{\"ok\":false,\"error\":\"Password must be 4 to 64 printable characters.\"}");
			return;
		}
		if (!is_valid_role(role)) {
			snprintf(g_json_resp, sizeof(g_json_resp),
				"{\"ok\":false,\"error\":\"Invalid role (must be admin or viewer).\"}");
			return;
		}

		idx = g_user_count++;
		snprintf(g_users[idx].username, sizeof(g_users[idx].username), "%s", username);
		snprintf(g_users[idx].role, sizeof(g_users[idx].role), "%s", role);
		set_user_password(&g_users[idx], password);
		save_users_file(1);
		build_users_list_json(g_json_resp, sizeof(g_json_resp), "User account created.");
		return;
	}

	if (strcmp(action, "update") == 0) {
		idx = find_user_idx(username);
		if (idx < 0) {
			snprintf(g_json_resp, sizeof(g_json_resp),
				"{\"ok\":false,\"error\":\"User account not found.\"}");
			return;
		}
		if (!is_valid_role(role)) {
			snprintf(g_json_resp, sizeof(g_json_resp),
				"{\"ok\":false,\"error\":\"Invalid role (must be admin or viewer).\"}");
			return;
		}
		if (strcmp(role, CONCARD_ROLE_VIEWER) == 0) {
			if (strcmp(username, prim_u) == 0) {
				snprintf(g_json_resp, sizeof(g_json_resp),
					"{\"ok\":false,\"error\":\"Primary system account cannot be demoted to Viewer.\"}");
				return;
			}
			if (strcmp(g_users[idx].role, CONCARD_ROLE_ADMIN) == 0 && count_admins() <= 1) {
				snprintf(g_json_resp, sizeof(g_json_resp),
					"{\"ok\":false,\"error\":\"Cannot demote the last remaining Administrator account.\"}");
				return;
			}
		}
		if (password && password[0]) {
			if (!is_valid_password(password)) {
				snprintf(g_json_resp, sizeof(g_json_resp),
					"{\"ok\":false,\"error\":\"Password must be 4 to 64 printable characters.\"}");
				return;
			}
			set_user_password(&g_users[idx], password);
			sync_primary_nvram_password_if_needed(username, password);
		}
		snprintf(g_users[idx].role, sizeof(g_users[idx].role), "%s", role);
		if (strcmp(username, g_req_user) == 0)
			snprintf(g_req_role, sizeof(g_req_role), "%s", role);
		save_users_file(1);
		build_users_list_json(g_json_resp, sizeof(g_json_resp), "User account updated.");
		return;
	}

	if (strcmp(action, "delete") == 0) {
		int k;
		idx = find_user_idx(username);
		if (idx < 0) {
			snprintf(g_json_resp, sizeof(g_json_resp),
				"{\"ok\":false,\"error\":\"User account not found.\"}");
			return;
		}
		if (strcmp(username, g_req_user) == 0) {
			snprintf(g_json_resp, sizeof(g_json_resp),
				"{\"ok\":false,\"error\":\"You cannot delete the account you are currently signed in with.\"}");
			return;
		}
		if (strcmp(username, prim_u) == 0) {
			snprintf(g_json_resp, sizeof(g_json_resp),
				"{\"ok\":false,\"error\":\"Primary system account cannot be deleted.\"}");
			return;
		}
		if (strcmp(g_users[idx].role, CONCARD_ROLE_ADMIN) == 0 && count_admins() <= 1) {
			snprintf(g_json_resp, sizeof(g_json_resp),
				"{\"ok\":false,\"error\":\"Cannot delete the last remaining Administrator account.\"}");
			return;
		}

		revoke_sessions_for_user(username);
		for (k = idx; k + 1 < g_user_count; k++)
			g_users[k] = g_users[k + 1];
		memset(&g_users[g_user_count - 1], 0, sizeof(g_users[0]));
		g_user_count--;
		save_users_file(1);
		build_users_list_json(g_json_resp, sizeof(g_json_resp), "User account deleted.");
		return;
	}

	snprintf(g_json_resp, sizeof(g_json_resp),
		"{\"ok\":false,\"error\":\"Unsupported user management action.\"}");
}

void do_concard_users_cgi(const char *url, FILE *stream)
{
	(void)url;
	if (!g_has_post_resp) {
		build_users_list_json(g_json_resp, sizeof(g_json_resp), "");
	}
	fputs(g_json_resp, stream);
	fflush(stream);
}
