#ifndef _CONCARD_AUTH_H_
#define _CONCARD_AUTH_H_

#include <stdio.h>

#define CONCARD_ROLE_ADMIN   "admin"
#define CONCARD_ROLE_VIEWER  "viewer"

void concard_auth_init(void);
void concard_auth_sync_nvram(void);
void concard_auth_begin_request(void);

int concard_auth_check_request(
	const char *cookie_hdr,
	const char *authorization_hdr,
	char *out_user,
	size_t out_user_sz,
	char *out_role,
	size_t out_role_sz
);

const char *concard_auth_get_extra_header(void);
int concard_auth_is_viewer_forbidden(const char *file, int method_id);

void do_concard_auth_post(const char *url, FILE *stream, int clen, char *boundary);
void do_concard_auth_cgi(const char *url, FILE *stream);

void do_concard_users_post(const char *url, FILE *stream, int clen, char *boundary);
void do_concard_users_cgi(const char *url, FILE *stream);

#endif /* _CONCARD_AUTH_H_ */
