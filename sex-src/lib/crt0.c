#include <stdint.h>

extern int main();
extern void sexos_exit(int status);

/**
 * SexOS User-Space Entry Point
 */
void _start() {
    // 1. Initialize environment (argc, argv, envp if needed)
    // For now, we assume no args
    
    // 2. Call main
    int ret = main();
    
    // 3. Exit
    sexos_exit(ret);
    
    // Should never reach here
    while (1) {
        __asm__ __volatile__ ("pause");
    }
}
