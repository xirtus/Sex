#ifndef _SDL_H
#define _SDL_H

#include <stdint.h>

typedef struct SDL_Window SDL_Window;

SDL_Window* SDL_CreateWindow(const char* title, int x, int y, int w, int h, uint32_t flags);
void SDL_UpdateWindowSurface(SDL_Window* window);
void SDL_Delay(uint32_t ms);
int SDL_Init(uint32_t flags);

#endif
