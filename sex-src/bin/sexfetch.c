#include <sexos.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/**
 * sexfetch: System Information Tool for SexOS
 */

int main() {
    SexSysInfo info;
    if (_syscall(SYS_SYSINFO, (uint64_t)&info, 0, 0) != 0) {
        printf("sexfetch: Failed to query system info.\n");
        return 1;
    }

    printf("\x1b[35m"); // Magenta
    printf("      _____\n");
    printf("     / ___ \\    \x1b[1m\x1b[37muser\x1b[0m@\x1b[1m\x1b[35msexos\x1b[0m\n");
    printf("    / /   \\ \\   -----------------\n");
    printf("   | |     | |  \x1b[1m\x1b[35mOS:\x1b[0m SexOS SASOS v0.1\n");
    printf("   | |     | |  \x1b[1m\x1b[35mKernel:\x1b[0m Sex Microkernel\n");
    printf("    \\ \\___/ /   \x1b[1m\x1b[35mUptime:\x1b[0m %llu seconds\n", info.uptime);
    printf("     \\_____/    \x1b[1m\x1b[35mPackages:\x1b[0m 12 (SPD)\n");
    printf("                \x1b[1m\x1b[35mShell:\x1b[0m sexsh v1.0\n");
    printf("                \x1b[1m\x1b[35mCPU:\x1b[0m %u x x86_64\n", info.cpu_count);
    printf("                \x1b[1m\x1b[35mMemory:\x1b[0m %llu MB / %llu MB\n", 
        info.used_ram / 1024 / 1024, info.total_ram / 1024 / 1024);
    printf("                \x1b[1m\x1b[35mPDs:\x1b[0m %u active\n", info.pd_count);
    printf("\x1b[0m\n");

    // Print color blocks
    for(int i=0; i<8; i++) printf("\x1b[4%dm  ", i);
    printf("\x1b[0m\n");

    return 0;
}
