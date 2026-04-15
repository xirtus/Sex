#ifndef _SYS_TERMIOS_H
#define _SYS_TERMIOS_H

#include <stdint.h>

typedef uint32_t tcflag_t;
typedef uint8_t  cc_t;
typedef uint32_t speed_t;

#define NCCS 32

struct termios {
    tcflag_t c_iflag;    /* input mode flags */
    tcflag_t c_oflag;    /* output mode flags */
    tcflag_t c_cflag;    /* control mode flags */
    tcflag_t c_lflag;    /* local mode flags */
    cc_t     c_line;     /* line discipline */
    cc_t     c_cc[NCCS]; /* control characters */
    speed_t  c_ispeed;   /* input speed */
    speed_t  c_ospeed;   /* output speed */
};

/* c_iflag bits */
#define IGNBRK  0000001
#define BRKINT  0000002
#define IGNPAR  0000004
#define PARMRK  0000010
#define INPCK   0000020
#define ISTRIP  0000040
#define INLCR   0000100
#define IGNCR   0000200
#define ICRNL   0000400
#define IXON    0002000
#define IXOFF   0010000

/* c_oflag bits */
#define OPOST   0000001
#define ONLCR   0000004

/* c_lflag bits */
#define ISIG    0000001
#define ICANON  0000002
#define ECHO    0000010
#define ECHOE   0000020
#define ECHOK   0000040
#define ECHONL  0000100
#define NOFLSH  0000200
#define TOSTOP  0000400
#define IEXTEN  0001000

/* tcsetattr uses */
#define TCSANOW   0
#define TCSADRAIN 1
#define TCSAFLUSH 2

#include <sys/ioctl.h>
#include <sexos.h>

static inline int tcgetattr(int fd, struct termios *termios_p) {
    return (int)_syscall(SYS_IOCTL, (uint64_t)fd, TCGETS, (uint64_t)termios_p);
}

static inline int tcsetattr(int fd, int optional_actions, const struct termios *termios_p) {
    // optional_actions is ignored for now, assuming TCSANOW
    return (int)_syscall(SYS_IOCTL, (uint64_t)fd, TCSETS, (uint64_t)termios_p);
}

#endif // _SYS_TERMIOS_H
