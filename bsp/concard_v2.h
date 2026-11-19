#ifndef _CONCARD_V2_H_
#define _CONCARD_V2_H_

#include <stdio.h>

void concard_v2_note_controller(const char *hdr_val);

void do_concard_telemetry_cgi(const char *url, FILE *stream);

void do_concard_locate_post(const char *url, FILE *stream, int clen, char *boundary);
void do_concard_locate_cgi(const char *url, FILE *stream);

void do_concard_ota_post(const char *url, FILE *stream, int clen, char *boundary);
void do_concard_ota_cgi(const char *url, FILE *stream);

#endif /* _CONCARD_V2_H_ */
