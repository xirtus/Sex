#include <stdio.h>
#include <stdlib.h>
#include <lua.h>

/**
 * Neovim Port for SexOS (Minimal)
 */

int main(int argc, char **argv) {
    printf("Neovim: SexOS Native Port\n");
    printf("Neovim: Initializing UI via sexdrm...\n");

    lua_State *L = luaL_newstate();
    luaL_openlibs(L);
    
    printf("Neovim: Starting Lua engine for plugins...\n");
    luaL_dostring(L, "print('Neovim core initialized.')");

    // Main loop would normally poll sexinput
    printf("Neovim: Ready. Enter command: ");
    
    lua_close(L);
    return 0;
}
