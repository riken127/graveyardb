use crate::domain::schema::model::{
    EnumType, Field, FieldConstraints, FieldType, PrimitiveType, Schema,
};
use crate::domain::schema::validation::{validate_event_payload, ValidationError};
use std::collections::HashMap;

#[test]
fn validates_required_and_min_constraints() {
    let mut fields = HashMap::new();
    fields.insert(
        "age".to_string(),
        Field {
            field_type: FieldType::Primitive(PrimitiveType::Number),
            nullable: false,
            overrides_on_null: false,
            constraints: Some(FieldConstraints {
                required: true,
                min_value: Some(18.0),
                ..Default::default()
            }),
        },
    );

    let schema = Schema {
        name: "User".to_string(),
        fields,
    };

    let json = serde_json::json!({ "age": 10 });
    let payload = serde_json::to_vec(&json).unwrap();
    let errors = validate_event_payload(&payload, &schema).unwrap_err();

    assert_eq!(errors.len(), 1);
    assert!(matches!(
        &errors[0],
        ValidationError::MinValue(field, value, min) if field == "age" && *value == 10.0 && *min == 18.0
    ));
}

#[test]
fn validates_regex_constraints() {
    let mut fields = HashMap::new();
    fields.insert(
        "username".to_string(),
        Field {
            field_type: FieldType::Primitive(PrimitiveType::String),
            nullable: false,
            overrides_on_null: false,
            constraints: Some(FieldConstraints {
                regex: Some("^[a-z]+$".to_string()),
                ..Default::default()
            }),
        },
    );

    let schema = Schema {
        name: "User".to_string(),
        fields,
    };

    let json = serde_json::json!({ "username": "Ada42" });
    let payload = serde_json::to_vec(&json).unwrap();
    let errors = validate_event_payload(&payload, &schema).unwrap_err();

    assert_eq!(errors.len(), 1);
    assert!(matches!(
        &errors[0],
        ValidationError::Regex(field, pattern) if field == "username" && pattern == "^[a-z]+$"
    ));
}

#[test]
fn validates_enum_array_and_nested_schema() {
    let mut address_fields = HashMap::new();
    address_fields.insert(
        "city".to_string(),
        Field {
            field_type: FieldType::Primitive(PrimitiveType::String),
            nullable: false,
            overrides_on_null: false,
            constraints: Some(FieldConstraints {
                required: true,
                ..Default::default()
            }),
        },
    );

    let address_schema = Schema {
        name: "Address".to_string(),
        fields: address_fields,
    };

    let mut root_fields = HashMap::new();
    root_fields.insert(
        "status".to_string(),
        Field {
            field_type: FieldType::Enum(EnumType {
                variants: vec!["active".to_string(), "inactive".to_string()],
            }),
            nullable: false,
            overrides_on_null: false,
            constraints: Some(FieldConstraints {
                required: true,
                ..Default::default()
            }),
        },
    );
    root_fields.insert(
        "tags".to_string(),
        Field {
            field_type: FieldType::Array(Box::new(FieldType::Primitive(PrimitiveType::String))),
            nullable: false,
            overrides_on_null: false,
            constraints: None,
        },
    );
    root_fields.insert(
        "address".to_string(),
        Field {
            field_type: FieldType::SubSchema(Box::new(address_schema)),
            nullable: false,
            overrides_on_null: false,
            constraints: Some(FieldConstraints {
                required: true,
                ..Default::default()
            }),
        },
    );

    let schema = Schema {
        name: "User".to_string(),
        fields: root_fields,
    };

    let json = serde_json::json!({
        "status": "disabled",
        "tags": ["ok", 42],
        "address": {}
    });
    let payload = serde_json::to_vec(&json).unwrap();
    let errors = validate_event_payload(&payload, &schema).unwrap_err();

    assert!(
        errors.iter().any(|e| matches!(
            e,
            ValidationError::InvalidEnumVariant(path, value)
            if path == "status" && value == "disabled"
        )),
        "expected enum validation error, got: {errors:?}"
    );
    assert!(
        errors.iter().any(|e| matches!(
            e,
            ValidationError::InvalidType(path)
            if path == "tags[1]"
        )),
        "expected array item type validation error, got: {errors:?}"
    );
    assert!(
        errors.iter().any(|e| matches!(
            e,
            ValidationError::MissingField(path)
            if path == "address.city"
        )),
        "expected nested required field validation error, got: {errors:?}"
    );
}

#[test]
fn rejects_null_for_non_nullable_fields() {
    let mut fields = HashMap::new();
    fields.insert(
        "name".to_string(),
        Field {
            field_type: FieldType::Primitive(PrimitiveType::String),
            nullable: false,
            overrides_on_null: false,
            constraints: None,
        },
    );

    let schema = Schema {
        name: "User".to_string(),
        fields,
    };

    let json = serde_json::json!({ "name": null });
    let payload = serde_json::to_vec(&json).unwrap();
    let errors = validate_event_payload(&payload, &schema).unwrap_err();

    assert_eq!(errors.len(), 1);
    assert!(matches!(
        &errors[0],
        ValidationError::NullNotAllowed(field) if field == "name"
    ));
}
