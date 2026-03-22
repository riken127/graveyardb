use crate::domain::schema::model::{FieldType, PrimitiveType, Schema};
use regex::Regex;
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Payload is not valid JSON")]
    InvalidJson(#[from] serde_json::Error),
    #[error("Field {0} is required but missing")]
    MissingField(String),
    #[error("Field {0} has invalid type")]
    InvalidType(String),
    #[error("Field {0} value {1} is less than min {2}")]
    MinValue(String, f64, f64),
    #[error("Field {0} value {1} is greater than max {2}")]
    MaxValue(String, f64, f64),
    #[error("Field {0} length {1} is less than min {2}")]
    MinLength(String, usize, i32),
    #[error("Field {0} length {1} is greater than max {2}")]
    MaxLength(String, usize, i32),
    #[error("Field {0} does not match regex {1}")]
    Regex(String, String),
    #[error("Field {0} must not be null")]
    NullNotAllowed(String),
    #[error("Field {0} has invalid enum variant {1}")]
    InvalidEnumVariant(String, String),
    #[error("Field {0} has invalid regex pattern {1}")]
    InvalidRegexPattern(String, String),
}

pub fn validate_event_payload(payload: &[u8], schema: &Schema) -> Result<(), Vec<ValidationError>> {
    let json_val: Value = match serde_json::from_slice(payload) {
        Ok(v) => v,
        Err(e) => return Err(vec![ValidationError::InvalidJson(e)]),
    };

    if !json_val.is_object() {
        return Err(vec![ValidationError::InvalidType("$".to_string())]);
    }

    let mut errors = Vec::new();
    validate_schema_value(&json_val, schema, "", &mut errors);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_schema_value(
    value: &Value,
    schema: &Schema,
    parent: &str,
    errors: &mut Vec<ValidationError>,
) {
    for (field_name, field_def) in &schema.fields {
        let path = field_path(parent, field_name);
        let val = value.get(field_name);

        let required = field_def
            .constraints
            .as_ref()
            .map(|constraints| constraints.required)
            .unwrap_or(false);

        let Some(val) = val else {
            if required {
                errors.push(ValidationError::MissingField(path));
            }
            continue;
        };

        if val.is_null() {
            if required {
                errors.push(ValidationError::MissingField(path));
            } else if !field_def.nullable {
                errors.push(ValidationError::NullNotAllowed(path));
            }
            continue;
        }

        let type_ok = validate_field_type(val, &field_def.field_type, &path, errors);

        if let Some(constraints) = &field_def.constraints {
            if type_ok {
                validate_constraints(val, &path, constraints, errors);
            }
        }
    }
}

fn validate_field_type(
    value: &Value,
    field_type: &FieldType,
    path: &str,
    errors: &mut Vec<ValidationError>,
) -> bool {
    match field_type {
        FieldType::Primitive(p) => {
            let valid = match p {
                PrimitiveType::String => value.is_string(),
                PrimitiveType::Number => value.is_number(),
                PrimitiveType::Boolean => value.is_boolean(),
            };

            if !valid {
                errors.push(ValidationError::InvalidType(path.to_string()));
            }

            valid
        }
        FieldType::Enum(enum_type) => {
            let Some(raw) = value.as_str() else {
                errors.push(ValidationError::InvalidType(path.to_string()));
                return false;
            };

            if !enum_type.variants.iter().any(|variant| variant == raw) {
                errors.push(ValidationError::InvalidEnumVariant(
                    path.to_string(),
                    raw.to_string(),
                ));
            }

            true
        }
        FieldType::Array(inner) => {
            let Some(values) = value.as_array() else {
                errors.push(ValidationError::InvalidType(path.to_string()));
                return false;
            };

            let mut all_valid = true;
            for (idx, item) in values.iter().enumerate() {
                let item_path = format!("{path}[{idx}]");
                if !validate_field_type(item, inner, &item_path, errors) {
                    all_valid = false;
                }
            }
            all_valid
        }
        FieldType::SubSchema(schema) => {
            if !value.is_object() {
                errors.push(ValidationError::InvalidType(path.to_string()));
                return false;
            }

            validate_schema_value(value, schema, path, errors);
            true
        }
    }
}

fn validate_constraints(
    value: &Value,
    path: &str,
    constraints: &crate::domain::schema::model::FieldConstraints,
    errors: &mut Vec<ValidationError>,
) {
    if let Some(n) = value.as_f64() {
        if let Some(min) = constraints.min_value {
            if n < min {
                errors.push(ValidationError::MinValue(path.to_string(), n, min));
            }
        }

        if let Some(max) = constraints.max_value {
            if n > max {
                errors.push(ValidationError::MaxValue(path.to_string(), n, max));
            }
        }
    }

    if let Some(s) = value.as_str() {
        let char_len = s.chars().count();

        if let Some(min) = constraints.min_length {
            if (char_len as i32) < min {
                errors.push(ValidationError::MinLength(path.to_string(), char_len, min));
            }
        }

        if let Some(max) = constraints.max_length {
            if (char_len as i32) > max {
                errors.push(ValidationError::MaxLength(path.to_string(), char_len, max));
            }
        }

        if let Some(pattern) = &constraints.regex {
            match Regex::new(pattern) {
                Ok(regex) => {
                    if !regex.is_match(s) {
                        errors.push(ValidationError::Regex(path.to_string(), pattern.clone()));
                    }
                }
                Err(_) => {
                    errors.push(ValidationError::InvalidRegexPattern(
                        path.to_string(),
                        pattern.clone(),
                    ));
                }
            }
        }
    }

    if let Some(arr) = value.as_array() {
        if let Some(min) = constraints.min_length {
            if (arr.len() as i32) < min {
                errors.push(ValidationError::MinLength(path.to_string(), arr.len(), min));
            }
        }

        if let Some(max) = constraints.max_length {
            if (arr.len() as i32) > max {
                errors.push(ValidationError::MaxLength(path.to_string(), arr.len(), max));
            }
        }
    }
}

fn field_path(parent: &str, field_name: &str) -> String {
    if parent.is_empty() {
        field_name.to_string()
    } else {
        format!("{parent}.{field_name}")
    }
}

// Include tests
#[cfg(test)]
#[path = "./validation_tests.rs"]
mod tests;
