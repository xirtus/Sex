#include <stdio.h>
#include <string.h>
#include <stdlib.h>

/**
 * zlib Port for SexOS (Functional Skeleton)
 */

typedef struct z_stream_s {
    const unsigned char *next_in;
    unsigned int avail_in;
    unsigned char *next_out;
    unsigned int avail_out;
    char *msg;
    void *state;
} z_stream;

int deflateInit(z_stream *strm, int level) {
    printf("zlib: Initializing deflate (level %d)\n", level);
    return 0; // Z_OK
}

int deflate(z_stream *strm, int flush) {
    // Basic "Store" implementation: copy in to out
    unsigned int count = (strm->avail_in < strm->avail_out) ? strm->avail_in : strm->avail_out;
    memcpy(strm->next_out, strm->next_in, count);
    strm->avail_in -= count;
    strm->avail_out -= count;
    strm->next_in += count;
    strm->next_out += count;
    return (strm->avail_in == 0) ? 1 : 0; // Z_STREAM_END or Z_OK
}

int deflateEnd(z_stream *strm) {
    printf("zlib: Deflate end.\n");
    return 0;
}

const char* zlibVersion() { return "1.3.1-sexos"; }
