#ifndef _SYS_IOCTL_H
#define _SYS_IOCTL_H

#include <stdint.h>

/**
 * SexOS Window Size structure.
 */
struct winsize {
    uint16_t ws_row;
    uint16_t ws_col;
    uint16_t ws_xpixel;
    uint16_t ws_ypixel;
};

/* Terminal IOCTLs */
#define TIOCGWINSZ 0x5413
#define TIOCSWINSZ 0x5414
#define TCGETS     0x5401
#define TCSETS     0x5402

#endif // _SYS_IOCTL_H
