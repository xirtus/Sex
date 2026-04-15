#ifndef _UV_H
#define _Z_UV_H

typedef struct uv_loop_s uv_loop_t;

int uv_loop_init(uv_loop_t *loop);
int uv_run(uv_loop_t *loop, int mode);
const char* uv_version_string();

#endif // _UV_H
