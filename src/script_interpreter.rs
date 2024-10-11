use rlua::{Lua, Result};

/// This function will initialize the Lua interpreter and run a script
pub fn run_lua_script(script: &str) -> Result<()> {
    let lua = Lua::new();  // Initialize Lua

    lua.context(|lua_ctx| {
        // Execute the provided Lua script
        lua_ctx.load(script).exec()?;
        Ok(())
    })
}