#include <stdio.h>
#include <lua.h>

int main() {
    printf("--- Lua Test ---\n");
    lua_State *L = luaL_newstate();
    luaL_openlibs(L);
    luaL_dostring(L, "print('Hello from Lua inside SexOS!')");
    lua_close(L);
    return 0;
}
