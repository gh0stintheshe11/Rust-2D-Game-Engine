#[cfg(test)]
mod tests {
    use rust_2d_game_engine::engine_gui::script_editor::check_script_syntax;

    #[test]
    fn test_valid_script_passes() {
        let script = r#"
            function update(scene_id, entity_id)
                local x = 1 + 2
                if x > 2 then
                    set_velocity(entity_id, 0.0, -1.0)
                end
            end
        "#;
        assert_eq!(check_script_syntax(script), None);
    }

    #[test]
    fn test_empty_script_passes() {
        assert_eq!(check_script_syntax(""), None);
    }

    #[test]
    fn test_syntax_error_reports_line() {
        // Missing `end` on line 4
        let script = "function update(a, b)\n    local x = 1\n    if x then\n";
        let error = check_script_syntax(script).expect("must be a syntax error");
        assert!(
            error.contains("line"),
            "error should carry a line reference: {}",
            error
        );
    }

    #[test]
    fn test_undefined_call_is_not_a_syntax_error() {
        // Unknown functions are a runtime concern; parsing must accept them
        let script = "function update(a, b)\n    totally_unknown_fn()\nend\n";
        assert_eq!(check_script_syntax(script), None);
    }
}
