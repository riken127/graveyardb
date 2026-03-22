use crate::domain::schema::model::{Field, FieldConstraints, FieldType, PrimitiveType, Schema};
use regex::Regex;
use std::collections::HashSet;

/// Errors produced while validating a schema definition itself.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ContractError {
    #[error("schema name must not be empty")]
    EmptySchemaName,
    #[error("schema {0} must define at least one field")]
    EmptySchemaFields(String),
    #[error("field name at path {0} must not be empty")]
    EmptyFieldName(String),
    #[error("field {0} is required and cannot be nullable")]
    RequiredNullableConflict(String),
    #[error("field {0} sets overrides_on_null but is not nullable")]
    OverridesOnNullRequiresNullable(String),
    #[error("field {0} has min_value {1} greater than max_value {2}")]
    InvalidNumericBounds(String, f64, f64),
    #[error("field {0} has negative min_length {1}")]
    NegativeMinLength(String, i32),
    #[error("field {0} has negative max_length {1}")]
    NegativeMaxLength(String, i32),
    #[error("field {0} has min_length {1} greater than max_length {2}")]
    InvalidLengthBounds(String, i32, i32),
    #[error("field {0} applies numeric constraints to a non-number type")]
    NumericConstraintsOnNonNumber(String),
    #[error("field {0} applies length constraints to an unsupported type")]
    LengthConstraintsOnUnsupportedType(String),
    #[error("field {0} applies regex to a non-string type")]
    RegexOnNonStringType(String),
    #[error("field {0} has invalid regex pattern {1}")]
    InvalidRegexPattern(String, String),
    #[error("enum field {0} must define at least one variant")]
    EmptyEnum(String),
    #[error("enum field {0} has empty variant at index {1}")]
    EmptyEnumVariant(String, usize),
    #[error("enum field {0} has duplicate variant {1}")]
    DuplicateEnumVariant(String, String),
}

/// Validates schema contract semantics before accepting a schema definition.
///
/// This validation is intentionally strict:
/// - constraints must be coherent (`min <= max`, no negative lengths),
/// - constraints must match field types (numeric only on numbers, regex on strings),
/// - enum definitions must be non-empty and duplicate-free.
pub fn validate_schema_contract(schema: &Schema) -> Result<(), Vec<ContractError>> {
    let mut errors = Vec::new();
    validate_schema(schema, "$", &mut errors);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_schema(schema: &Schema, schema_path: &str, errors: &mut Vec<ContractError>) {
    if schema.name.trim().is_empty() {
        errors.push(ContractError::EmptySchemaName);
    }
    if schema.fields.is_empty() {
        errors.push(ContractError::EmptySchemaFields(schema_path.to_string()));
    }

    for (field_name, field) in &schema.fields {
        if field_name.trim().is_empty() {
            errors.push(ContractError::EmptyFieldName(schema_path.to_string()));
            continue;
        }

        let path = if schema_path == "$" {
            field_name.to_string()
        } else {
            format!("{schema_path}.{field_name}")
        };

        validate_field(path, field, errors);
    }
}

fn validate_field(path: String, field: &Field, errors: &mut Vec<ContractError>) {
    validate_field_type_definition(&path, &field.field_type, errors);

    if field.overrides_on_null && !field.nullable {
        errors.push(ContractError::OverridesOnNullRequiresNullable(path.clone()));
    }

    if let Some(constraints) = &field.constraints {
        validate_constraints_definition(&path, constraints, field, errors);
    }
}

fn validate_field_type_definition(
    path: &str,
    field_type: &FieldType,
    errors: &mut Vec<ContractError>,
) {
    match field_type {
        FieldType::Enum(enum_type) => {
            if enum_type.variants.is_empty() {
                errors.push(ContractError::EmptyEnum(path.to_string()));
                return;
            }

            let mut seen = HashSet::new();
            for (idx, variant) in enum_type.variants.iter().enumerate() {
                let trimmed = variant.trim();
                if trimmed.is_empty() {
                    errors.push(ContractError::EmptyEnumVariant(path.to_string(), idx));
                    continue;
                }
                if !seen.insert(trimmed.to_string()) {
                    errors.push(ContractError::DuplicateEnumVariant(
                        path.to_string(),
                        trimmed.to_string(),
                    ));
                }
            }
        }
        FieldType::Array(inner) => {
            let inner_path = format!("{path}[]");
            validate_field_type_definition(&inner_path, inner, errors);
        }
        FieldType::SubSchema(schema) => {
            validate_schema(schema, path, errors);
        }
        FieldType::Primitive(_) => {}
    }
}

fn validate_constraints_definition(
    path: &str,
    constraints: &FieldConstraints,
    field: &Field,
    errors: &mut Vec<ContractError>,
) {
    if constraints.required && field.nullable {
        errors.push(ContractError::RequiredNullableConflict(path.to_string()));
    }

    let has_numeric = constraints.min_value.is_some() || constraints.max_value.is_some();
    let has_length = constraints.min_length.is_some() || constraints.max_length.is_some();
    let has_regex = constraints
        .regex
        .as_ref()
        .map(|value| !value.is_empty())
        .unwrap_or(false);

    if let (Some(min), Some(max)) = (constraints.min_value, constraints.max_value) {
        if min > max {
            errors.push(ContractError::InvalidNumericBounds(
                path.to_string(),
                min,
                max,
            ));
        }
    }

    if let Some(min_len) = constraints.min_length {
        if min_len < 0 {
            errors.push(ContractError::NegativeMinLength(path.to_string(), min_len));
        }
    }
    if let Some(max_len) = constraints.max_length {
        if max_len < 0 {
            errors.push(ContractError::NegativeMaxLength(path.to_string(), max_len));
        }
    }
    if let (Some(min_len), Some(max_len)) = (constraints.min_length, constraints.max_length) {
        if min_len > max_len {
            errors.push(ContractError::InvalidLengthBounds(
                path.to_string(),
                min_len,
                max_len,
            ));
        }
    }

    if has_numeric
        && !matches!(
            field.field_type,
            FieldType::Primitive(PrimitiveType::Number)
        )
    {
        errors.push(ContractError::NumericConstraintsOnNonNumber(
            path.to_string(),
        ));
    }

    let supports_length = matches!(
        field.field_type,
        FieldType::Primitive(PrimitiveType::String) | FieldType::Enum(_) | FieldType::Array(_)
    );
    if has_length && !supports_length {
        errors.push(ContractError::LengthConstraintsOnUnsupportedType(
            path.to_string(),
        ));
    }

    let supports_regex = matches!(
        field.field_type,
        FieldType::Primitive(PrimitiveType::String) | FieldType::Enum(_)
    );
    if has_regex && !supports_regex {
        errors.push(ContractError::RegexOnNonStringType(path.to_string()));
    }

    if let Some(pattern) = &constraints.regex {
        if !pattern.is_empty() && Regex::new(pattern).is_err() {
            errors.push(ContractError::InvalidRegexPattern(
                path.to_string(),
                pattern.clone(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{validate_schema_contract, ContractError};
    use crate::domain::schema::model::{Field, FieldConstraints, FieldType, PrimitiveType, Schema};
    use std::collections::HashMap;

    fn field(
        field_type: FieldType,
        nullable: bool,
        constraints: Option<FieldConstraints>,
    ) -> Field {
        Field {
            field_type,
            nullable,
            overrides_on_null: false,
            constraints,
        }
    }

    #[test]
    fn accepts_valid_contract() {
        let mut fields = HashMap::new();
        fields.insert(
            "email".to_string(),
            field(
                FieldType::Primitive(PrimitiveType::String),
                false,
                Some(FieldConstraints {
                    required: true,
                    min_length: Some(5),
                    max_length: Some(256),
                    regex: Some("^[^@\\s]+@[^@\\s]+\\.[^@\\s]+$".to_string()),
                    ..Default::default()
                }),
            ),
        );
        fields.insert(
            "scores".to_string(),
            field(
                FieldType::Array(Box::new(FieldType::Primitive(PrimitiveType::Number))),
                true,
                Some(FieldConstraints {
                    min_length: Some(1),
                    max_length: Some(10),
                    ..Default::default()
                }),
            ),
        );

        let schema = Schema {
            name: "UserContract".to_string(),
            fields,
        };

        let result = validate_schema_contract(&schema);
        assert!(result.is_ok(), "unexpected validation errors: {result:?}");
    }

    #[test]
    fn rejects_invalid_constraint_combinations() {
        let mut fields = HashMap::new();
        fields.insert(
            "age".to_string(),
            field(
                FieldType::Primitive(PrimitiveType::String),
                true,
                Some(FieldConstraints {
                    required: true,
                    min_value: Some(10.0),
                    max_value: Some(5.0),
                    ..Default::default()
                }),
            ),
        );

        let schema = Schema {
            name: "BrokenContract".to_string(),
            fields,
        };

        let errors =
            validate_schema_contract(&schema).expect_err("expected schema contract errors");
        assert!(errors
            .iter()
            .any(|e| matches!(e, ContractError::RequiredNullableConflict(path) if path == "age")));
        assert!(errors.iter().any(|e| matches!(
            e,
            ContractError::NumericConstraintsOnNonNumber(path) if path == "age"
        )));
        assert!(errors.iter().any(|e| matches!(
            e,
            ContractError::InvalidNumericBounds(path, min, max)
            if path == "age" && (*min - 10.0).abs() < f64::EPSILON && (*max - 5.0).abs() < f64::EPSILON
        )));
    }

    #[test]
    fn rejects_invalid_regex_and_lengths() {
        let mut fields = HashMap::new();
        fields.insert(
            "tags".to_string(),
            field(
                FieldType::Array(Box::new(FieldType::Primitive(PrimitiveType::String))),
                true,
                Some(FieldConstraints {
                    min_length: Some(-1),
                    max_length: Some(2),
                    regex: Some("[".to_string()),
                    ..Default::default()
                }),
            ),
        );

        let schema = Schema {
            name: "BadRegexContract".to_string(),
            fields,
        };

        let errors =
            validate_schema_contract(&schema).expect_err("expected schema contract errors");
        assert!(errors.iter().any(|e| matches!(
            e,
            ContractError::NegativeMinLength(path, -1) if path == "tags"
        )));
        assert!(errors.iter().any(|e| matches!(
            e,
            ContractError::RegexOnNonStringType(path) if path == "tags"
        )));
    }
}
