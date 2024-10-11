#[cfg(test)]
mod tests {
    use rust_2d_game_engine::interpreter;
    use pyo3::prelude::*;
    use std::collections::HashMap;

    #[test]
    fn test_run_simple_script_without_variables() {
        // Test running a basic Python script without passing any variables
        let script = r#"
result = 10 + 20
result
"#;
        Python::with_gil(|py| {
            let result = interpreter::run_python_script(script, None).unwrap();
            let extracted_result: i32 = result.extract(py).unwrap();
            assert_eq!(extracted_result, 30);
        });
    }

    #[test]
    fn test_run_script_with_variables() {
        // Test running a Python script with variables passed from Rust
        let script = r#"
result = x + y
result
"#;

        Python::with_gil(|py| {
            // Create variables (x and y) to pass to the script
            let mut variables = HashMap::new();
            variables.insert("x".to_string(), 5.to_object(py));
            variables.insert("y".to_string(), 15.to_object(py));

            // Run the script and pass variables
            let result = interpreter::run_python_script(script, Some(variables)).unwrap();
            let extracted_result: i32 = result.extract(py).unwrap();
            assert_eq!(extracted_result, 20);
        });
    }

    #[test]
    fn test_run_script_with_string_return() {
        // Test running a Python script that returns a string
        let script = r#"
"Hello from Python!"
"#;
        Python::with_gil(|py| {
            let result = interpreter::run_python_script(script, None).unwrap();
            let extracted_result: String = result.extract(py).unwrap();
            assert_eq!(extracted_result, "Hello from Python!");
        });
    }

    #[test]
    fn test_run_script_with_list_return() {
        // Test running a Python script that returns a list
        let script = r#"
[1, 2, 3, 4]
"#;
        Python::with_gil(|py| {
            let result = interpreter::run_python_script(script, None).unwrap();
            let list: Vec<i32> = result.extract(py).unwrap();
            assert_eq!(list, vec![1, 2, 3, 4]);
        });
    }

    #[test]
    fn test_run_script_with_error() {
        // Test running a Python script that triggers an error (division by zero)
        let script = r#"
result = 10 / 0
"#;
        let result = interpreter::run_python_script(script, None);
        assert!(result.is_err(), "Expected an error due to division by zero");
    }

    #[test]
    fn test_run_script_with_complex_variables() {
        // Test passing complex variables like lists and dictionaries
        let script = r#"
sum(x) + y["key"]
"#;

        Python::with_gil(|py| {
            // Create variables to pass to the script
            let mut variables = HashMap::new();
            variables.insert("x".to_string(), vec![1, 2, 3, 4].to_object(py));
            let mut dict = HashMap::new();
            dict.insert("key", 10);
            variables.insert("y".to_string(), dict.to_object(py));

            // Run the script and check the result
            let result = interpreter::run_python_script(script, Some(variables)).unwrap();
            let extracted_result: i32 = result.extract(py).unwrap();
            assert_eq!(extracted_result, 20);
        });
    }
}