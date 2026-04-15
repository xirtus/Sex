#include <sexos.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/**
 * sexsh: The Native SexOS Shell
 * Features: sex-gemini predictive typing, ANSI colors, command execution.
 */

void print_prompt() {
    printf("\x1b[1m\x1b[35msexsh\x1b[0m \x1b[37m>\x1b[0m ");
}

void get_prediction(const char* input, char* prediction) {
    // In a real system, this would call the sex-gemini PDX service.
    // For the prototype, we use a simple heuristic.
    if (strncmp(input, "sex", 3) == 0) strcpy(prediction, "fetch");
    else if (strncmp(input, "ls", 2) == 0) strcpy(prediction, " /bin");
    else if (strncmp(input, "help", 4) == 0) strcpy(prediction, " commands");
    else prediction[0] = '\0';
}

int main() {
    char cmd[256];
    char prediction[256];
    int pos = 0;

    printf("Welcome to \x1b[1mSexShell v1.0\x1b[0m (Powered by Gemini AI)\n");
    printf("Type 'help' for commands.\n\n");

    while (1) {
        print_prompt();
        pos = 0;
        memset(cmd, 0, sizeof(cmd));

        while (1) {
            char c;
            if (sexos_read(0, &c, 1) > 0) {
                if (c == '\n' || c == '\r') {
                    printf("\n");
                    break;
                } else if (c == 8 || c == 127) { // Backspace
                    if (pos > 0) {
                        pos--;
                        cmd[pos] = '\0';
                        printf("\b \b");
                    }
                } else if (c == '\t') { // Tab completion / Accept prediction
                    get_prediction(cmd, prediction);
                    if (prediction[0] != '\0') {
                        printf("%s", prediction);
                        strcat(cmd, prediction);
                        pos += strlen(prediction);
                    }
                } else {
                    cmd[pos++] = c;
                    printf("%c", c);
                    
                    // Show gray predictive text (like kitty/zsh-autosuggestions)
                    get_prediction(cmd, prediction);
                    if (prediction[0] != '\0') {
                        printf("\x1b[90m%s\x1b[0m", prediction);
                        // Move cursor back
                        for(int i=0; i<strlen(prediction); i++) printf("\b");
                    }
                }
            }
            sexos_yield();
        }

        if (strlen(cmd) == 0) continue;

        if (strcmp(cmd, "help") == 0) {
            printf("Available commands: help, clear, sexfetch, exit, pstat\n");
        } else if (strcmp(cmd, "clear") == 0) {
            printf("\x1b[2J\x1b[H");
        } else if (strcmp(cmd, "sexfetch") == 0) {
            // In a real system, we'd spawn_pd. For the prototype, we link or simulate.
            printf("Spawning sexfetch...\n");
            _syscall(SYS_SPAWN_PD, (uint64_t)"/bin/sexfetch.sex", 0, 0);
        } else if (strcmp(cmd, "exit") == 0) {
            break;
        } else {
            printf("sexsh: command not found: %s\n", cmd);
        }
    }

    return 0;
}
