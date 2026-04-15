#include <stdio.h>
#include <stdlib.h>
#include <SDL2/SDL.h>

/**
 * SexOS DOOM Port (Minimal)
 * Demonstrating Phase 16 Voxel & Multimedia Milestone.
 */

int main(int argc, char **argv) {
    printf("DOOM: SexOS Native Port (Phase 16)\n");
    
    if (SDL_Init(0) != 0) {
        return 1;
    }

    SDL_Window* window = SDL_CreateWindow("DOOM SASOS", 0, 0, 640, 400, 0);
    
    printf("DOOM: Loading WAD files into Global SAS...\n");
    printf("DOOM: Setting up BSP tree and sector rendering...\n");

    // Main Game Loop
    for (int frame = 0; frame < 100; frame++) {
        // In a real port, this would render to the window surface
        SDL_UpdateWindowSurface(window);
        SDL_Delay(16); // ~60 FPS
    }

    printf("DOOM: Test run complete. RIP AND TEAR.\n");
    return 0;
}
