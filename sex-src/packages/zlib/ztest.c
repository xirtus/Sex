#include <stdio.h>
#include <stdlib.h>
#include <zlib.h>

int main() {
    printf("--- zlib Test ---\n");
    printf("Version: %s\n", zlibVersion());

    const char *data = "SexOS is fast!";
    unsigned char buffer[128];
    unsigned long len = 128;

    if (compress(buffer, &len, (const unsigned char*)data, 15) == 0) {
        printf("Compression SUCCESS. Output length: %lu\n", len);
    }

    return 0;
}
