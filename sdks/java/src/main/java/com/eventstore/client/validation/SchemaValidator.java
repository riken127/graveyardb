package com.eventstore.client.validation;

import com.eventstore.client.model.Field;
import com.eventstore.client.model.FieldConstraints;
import com.eventstore.client.model.FieldType;
import com.eventstore.client.model.Schema;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;

import java.io.IOException;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.regex.Pattern;

/**
 * Client-side schema validator used for preflight payload checks.
 * <p>
 * This helper validates payloads against the SDK schema model before RPC calls.
 * The server remains the authoritative validator.
 */
public final class SchemaValidator {

    private static final ObjectMapper OBJECT_MAPPER = new ObjectMapper();

    private SchemaValidator() {
    }

    /**
     * Validates a JSON payload against {@code schema}.
     *
     * @param payload JSON payload bytes.
     * @param schema schema definition to validate against.
     * @return validation error list. Empty list means the payload is valid.
     */
    public static List<String> validate(byte[] payload, Schema schema) {
        List<String> errors = new ArrayList<>();
        JsonNode root;
        try {
            root = OBJECT_MAPPER.readTree(payload);
        } catch (IOException e) {
            errors.add("Invalid JSON payload: " + e.getMessage());
            return errors;
        }

        if (!root.isObject()) {
            errors.add("Payload root must be a JSON object");
            return errors;
        }

        validateObject(root, schema, "", errors);
        return errors;
    }

    private static void validateObject(JsonNode objectNode, Schema schema, String parentPath, List<String> errors) {
        for (Map.Entry<String, Field> entry : schema.getFieldsMap().entrySet()) {
            String fieldName = entry.getKey();
            Field fieldDef = entry.getValue();
            String fieldPath = qualify(parentPath, fieldName);
            JsonNode node = objectNode.get(fieldName);

            boolean required = fieldDef.hasConstraints() && fieldDef.getConstraints().getRequired();
            boolean nullable = fieldDef.getNullable();

            if (node == null || node.isNull()) {
                if (required || !nullable) {
                    errors.add(String.format("Field '%s' is required but missing or null", fieldPath));
                }
                continue;
            }

            validateFieldType(fieldPath, node, fieldDef.getFieldType(), errors);
            if (fieldDef.hasConstraints()) {
                validateConstraints(fieldPath, node, fieldDef.getConstraints(), errors);
            }
        }
    }

    private static void validateFieldType(String fieldPath, JsonNode node, FieldType fieldType, List<String> errors) {
        switch (fieldType.getKindCase()) {
            case PRIMITIVE:
                validatePrimitive(fieldPath, node, fieldType.getPrimitive(), errors);
                break;
            case ENUM_DEF:
                if (!node.isTextual()) {
                    errors.add(String.format("Field '%s' must be a STRING enum value", fieldPath));
                    return;
                }
                String value = node.asText();
                if (!fieldType.getEnumDef().getVariantsList().contains(value)) {
                    errors.add(String.format(
                            "Field '%s' value '%s' is not a valid enum variant %s",
                            fieldPath,
                            value,
                            fieldType.getEnumDef().getVariantsList()));
                }
                break;
            case ARRAY_DEF:
                if (!node.isArray()) {
                    errors.add(String.format("Field '%s' must be an ARRAY", fieldPath));
                    return;
                }

                FieldType elementType = fieldType.getArrayDef().getElementType();
                for (int i = 0; i < node.size(); i++) {
                    validateFieldType(fieldPath + "[" + i + "]", node.get(i), elementType, errors);
                }
                break;
            case SUB_SCHEMA:
                if (!node.isObject()) {
                    errors.add(String.format("Field '%s' must be an OBJECT", fieldPath));
                    return;
                }
                validateObject(node, fieldType.getSubSchema(), fieldPath, errors);
                break;
            case KIND_NOT_SET:
                errors.add(String.format("Field '%s' has no schema type metadata", fieldPath));
                break;
            default:
                break;
        }
    }

    private static void validatePrimitive(
            String fieldPath,
            JsonNode node,
            FieldType.Primitive primitiveType,
            List<String> errors) {
        switch (primitiveType) {
            case STRING:
                if (!node.isTextual()) {
                    errors.add(String.format("Field '%s' must be a STRING", fieldPath));
                }
                break;
            case NUMBER:
                if (!node.isNumber()) {
                    errors.add(String.format("Field '%s' must be a NUMBER", fieldPath));
                }
                break;
            case BOOLEAN:
                if (!node.isBoolean()) {
                    errors.add(String.format("Field '%s' must be a BOOLEAN", fieldPath));
                }
                break;
            default:
                break;
        }
    }

    private static void validateConstraints(
            String fieldPath,
            JsonNode node,
            FieldConstraints constraints,
            List<String> errors) {
        if (node.isNumber()) {
            double value = node.asDouble();
            if (constraints.hasMinValue() && value < constraints.getMinValue()) {
                errors.add(String.format(
                        "Field '%s' value %f is less than min %f",
                        fieldPath,
                        value,
                        constraints.getMinValue()));
            }
            if (constraints.hasMaxValue() && value > constraints.getMaxValue()) {
                errors.add(String.format(
                        "Field '%s' value %f is greater than max %f",
                        fieldPath,
                        value,
                        constraints.getMaxValue()));
            }
        }

        if (node.isTextual()) {
            String value = node.asText();
            if (constraints.hasMinLength() && value.length() < constraints.getMinLength()) {
                errors.add(String.format(
                        "Field '%s' length %d is less than min %d",
                        fieldPath,
                        value.length(),
                        constraints.getMinLength()));
            }
            if (constraints.hasMaxLength() && value.length() > constraints.getMaxLength()) {
                errors.add(String.format(
                        "Field '%s' length %d is greater than max %d",
                        fieldPath,
                        value.length(),
                        constraints.getMaxLength()));
            }
            if (constraints.hasRegex()) {
                String regex = constraints.getRegex();
                if (!regex.isEmpty() && !Pattern.matches(regex, value)) {
                    errors.add(String.format(
                            "Field '%s' value '%s' does not match regex '%s'",
                            fieldPath,
                            value,
                            regex));
                }
            }
        }
    }

    private static String qualify(String parentPath, String fieldName) {
        if (parentPath == null || parentPath.isEmpty()) {
            return fieldName;
        }
        return parentPath + "." + fieldName;
    }
}
