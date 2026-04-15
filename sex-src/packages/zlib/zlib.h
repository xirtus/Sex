#ifndef _ZLIB_H
#define _ZLIB_H

#include <stddef.h>

const char* zlibVersion();
int compress(unsigned char *dest, unsigned long *destLen, const unsigned char *source, unsigned long sourceLen);
int uncompress(unsigned char *dest, unsigned long *destLen, const unsigned char *source, unsigned long sourceLen);

#endif // _ZLIB_H
