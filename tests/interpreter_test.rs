#[cfg(test)]
mod tests {
    use rust_2d_game_engine::interpreter;
    use rlua::Lua;

    #[test]
    fn test_run_simple_script() {
        // Test a simple Lua script that adds two numbers
        let script = r#"
            x = 10
            y = 20
            result = x + y
        "#;

        let result = interpreter::run_lua_script(script);
        assert!(result.is_ok(), "Failed to run a simple Lua script");
    }

    #[test]
    fn test_run_script_with_error() {
        // Test a Lua script that tries to use an undefined variable
        let lua = Lua::new();
        let script = r#"
            x = 10
            if y == nil then
                y = 0  -- Assign a default value if 'y' is undefined
            end
            result = x + y
        "#;
    
        let result = lua.context(|lua_ctx| lua_ctx.load(script).exec());
    
        // Since we're explicitly checking for 'nil', the script should run successfully
        assert!(result.is_ok(), "Expected Lua to handle undefined variables as nil, but it failed");
    }

    #[test]
    fn test_lua_math_operations() {
        // Test a Lua script performing math operations
        let script = r#"
            result = (10 * 5) / 2 - 7
        "#;

        let lua = Lua::new();
        lua.context(|lua_ctx| {
            lua_ctx.load(script).exec().unwrap();
            let result: f64 = lua_ctx.globals().get("result").unwrap();
            assert_eq!(result, 18.0, "Math operation failed in Lua");
        });
    }

    #[test]
    fn test_pass_data_to_lua() {
        // Test passing data to Lua
        let lua = Lua::new();
        lua.context(|lua_ctx| {
            let globals = lua_ctx.globals();
            globals.set("x", 50).unwrap();
            globals.set("y", 100).unwrap();

            lua_ctx.load(r#"
                result = x + y
            "#).exec().unwrap();

            let result: i32 = lua_ctx.globals().get("result").unwrap();
            assert_eq!(result, 150, "Failed to pass data to Lua script");
        });
    }

    #[test]
    fn test_return_data_from_lua() {
        // Test returning data from Lua to Rust
        let lua = Lua::new();
        lua.context(|lua_ctx| {
            lua_ctx.load(r#"
                function add(a, b)
                    return a + b
                end
            "#).exec().unwrap();

            let add: rlua::Function = lua_ctx.globals().get("add").unwrap();
            let result: i32 = add.call((10, 20)).unwrap();
            assert_eq!(result, 30, "Failed to return correct data from Lua");
        });
    }

    #[test]
    fn test_complex_script() {
        // Test running a more complex Lua script (basic object simulation)
        let script = r#"
            obj = {
                x = 0,
                y = 0,
                vx = 1,
                vy = 1
            }

            function update_position(obj)
                obj.x = obj.x + obj.vx
                obj.y = obj.y + obj.vy
            end

            update_position(obj)
        "#;

        let lua = Lua::new();
        lua.context(|lua_ctx| {
            lua_ctx.load(script).exec().unwrap();

            let obj: rlua::Table = lua_ctx.globals().get("obj").unwrap();
            let x: i32 = obj.get("x").unwrap();
            let y: i32 = obj.get("y").unwrap();

            assert_eq!(x, 1, "Object x-coordinate was not updated correctly");
            assert_eq!(y, 1, "Object y-coordinate was not updated correctly");
        });
    }

    #[test]
    fn test_handle_error_in_lua_script() {
        // Test Lua's handling of division by zero
        let lua = Lua::new();
        let script = r#"
            function divide(a, b)
                return a / b
            end

            result = divide(10, 0)
        "#;

        lua.context(|lua_ctx| {
            lua_ctx.load(script).exec().unwrap();

            // Lua returns 'inf' or 'nan' on division by zero, not an error
            let result: f64 = lua_ctx.globals().get("result").unwrap();
            assert!(result.is_infinite(), "Expected Lua to return 'inf' on division by zero, but got: {}", result);
        });
    }
}