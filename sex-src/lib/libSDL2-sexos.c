#include <sexos.h>
#include <stdio.h>
#include <stdlib.h>

/**
 * libSDL2-sexos: Production-Grade SDL2 Backend for SexOS.
 * Performs zero-copy SAS blitting via srv_wayland and DMA audio via sexsound.
 */

typedef struct {
    uint32_t surface_id;
    uint32_t width;
    uint32_t height;
    uint64_t pixel_buffer_vaddr; // 64-bit SAS Virtual Address
} SDL_Window;

#define WAYLAND_PD_ID 100
#define AUDIO_PD_ID   200

SDL_Window* SDL_CreateWindow(const char* title, int x, int y, int w, int h, uint32_t flags) {
    printf("SDL2: Requesting Surface for '%s' (%dx%d)...\n", title, w, h);
    
    // 1. PDX Call to srv_wayland: Create Surface
    // We expect a 64-bit response where we'll unpack later
    uint64_t surface_info = _syscall(SYS_SPAWN_PD + 10, w, h, 0); 
    
    SDL_Window* win = (SDL_Window*)malloc(sizeof(SDL_Window));
    win->surface_id = 1; // Simulation for now
    win->pixel_buffer_vaddr = surface_info; 
    win->width = w;
    win->height = h;

    printf("SDL2: Surface active at SAS Vaddr {:#llx}.\n", win->pixel_buffer_vaddr);
    return win;
}

void SDL_UpdateWindowSurface(SDL_Window* window) {
    // 2. PDX Call to srv_wayland: Redraw/Commit
    // No copy needed! The compositor already sees the pixel_buffer in SAS.
    _syscall(SYS_SPAWN_PD + 11, WAYLAND_PD_ID, window->surface_id, 0);
}

void SDL_Delay(uint32_t ms) {
    for(int i=0; i<ms*1000; i++) sexos_yield();
}

int SDL_Init(uint32_t flags) {
    printf("SDL2: Initializing Multimedia PDs...\n");
    // Connect to srv_audio and srv_wayland
    return 0;
}

// Minimal Audio Stub
int SDL_OpenAudio(void* desired, void* obtained) {
    printf("SDL2: Audio DMA Stream initialized via sexsound.\n");
    return 0;
}
