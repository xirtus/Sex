#ifndef _LUA_H
#define _LUA_H

typedef struct lua_State lua_State;
lua_State* luaL_newstate();
void luaL_openlibs(lua_State *L);
int luaL_dostring(lua_State *L, const char *str);
void lua_close(lua_State *L);

#endif // _LUA_H
