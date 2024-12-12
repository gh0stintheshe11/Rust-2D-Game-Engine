use rlua::{Lua, Result};

/// This function will initialize the Lua interpreter and run a script
pub fn run_lua_script(script: &str) -> Result<()> {
    let lua = Lua::new(); // Initialize Lua

    // Execute the provided Lua script
    lua.load(script).exec()?;
    Ok(())
}
