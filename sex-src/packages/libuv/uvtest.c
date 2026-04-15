#include <stdio.h>
#include <uv.h>

int main() {
    printf("--- libuv Test ---\n");
    printf("Version: %s\n", uv_version_string());

    uv_loop_t loop;
    uv_loop_init(&loop);
    uv_run(&loop, 0);

    return 0;
}
