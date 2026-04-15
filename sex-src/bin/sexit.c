#include <sexos.h>
#include <stdio.h>

/**
 * sexit: The System Supervisor (PID 1)
 */
int main() {
    printf("SEXIT: System Supervisor active.\n");
    while(1) sexos_yield();
    return 0;
}
