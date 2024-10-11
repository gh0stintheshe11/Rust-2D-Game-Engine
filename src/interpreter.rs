use pyo3::prelude::*;
use std::collections::HashMap;
use pyo3::types::PyDict;

/// A general function that runs a Python script and optionally passes in variables
/// `script`: the Python code to be executed
/// `variables`: a map of variables to pass into the Python script (optional)
/// Returns: The result of the script execution
pub fn run_python_script(script: &str, variables: Option<HashMap<String, PyObject>>) -> PyResult<PyObject> {
    Python::with_gil(|py| {
        // Create an empty dictionary to hold variables if provided
        let locals = PyDict::new(py);

        // If variables were provided, insert them into the Python `locals` dictionary
        if let Some(vars) = variables {
            for (key, value) in vars {
                locals.set_item(key, value)?;
            }
        }

        // Run the provided Python script with the optional variables
        let result: PyObject = py.eval(script, None, Some(&locals))?.into();

        // Return the result of the script execution
        Ok(result)
    })
}