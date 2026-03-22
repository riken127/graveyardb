package com.eventstore.client.schema;

import com.eventstore.client.annotations.GraveyardEntity;
import com.eventstore.client.annotations.GraveyardField;
import com.eventstore.client.model.Field;
import com.eventstore.client.model.FieldConstraints;
import com.eventstore.client.model.FieldType;
import com.eventstore.client.model.Schema;

import java.util.Collection;
import java.lang.reflect.Modifier;

public class SchemaGenerator {

    /**
     * Generates a Proto Schema from a Java class annotated with @GraveyardEntity.
     * <p>
     * Declared instance fields are exported into the schema. Fields annotated with
     * {@link GraveyardField} contribute nullability and constraint metadata, while
     * unannotated fields default to nullable/unconstrained.
     *
     * @param clazz The entity class.
     * @return The generated Schema.
     */
    public static Schema generate(Class<?> clazz) {
        GraveyardEntity entityAuth = clazz.getAnnotation(GraveyardEntity.class);
        if (entityAuth == null) {
            throw new IllegalArgumentException("Class must be annotated with @GraveyardEntity");
        }

        Schema.Builder schemaBuilder = Schema.newBuilder();
        schemaBuilder.setName(entityAuth.value());

        for (java.lang.reflect.Field reflectField : clazz.getDeclaredFields()) {
            if (reflectField.isSynthetic()
                    || Modifier.isStatic(reflectField.getModifiers())
                    || Modifier.isTransient(reflectField.getModifiers())) {
                continue;
            }

            Field protoField = generateField(reflectField);
            if (protoField != null) {
                schemaBuilder.putFields(reflectField.getName(), protoField);
            }
        }

        return schemaBuilder.build();
    }

    private static Field generateField(java.lang.reflect.Field reflectField) {
        Field.Builder fieldBuilder = Field.newBuilder();
        GraveyardField fieldAuth = reflectField.getAnnotation(GraveyardField.class);

        if (fieldAuth != null) {
            fieldBuilder.setNullable(fieldAuth.nullable());
            fieldBuilder.setOverridesOnNull(fieldAuth.overridesOnNull());

            // Constraints
            FieldConstraints.Builder constraintsBuilder = FieldConstraints.newBuilder();
            boolean hasConstraints = false;

            if (!Double.isNaN(fieldAuth.min())) {
                constraintsBuilder.setMinValue(fieldAuth.min());
                hasConstraints = true;
            }
            if (!Double.isNaN(fieldAuth.max())) {
                constraintsBuilder.setMaxValue(fieldAuth.max());
                hasConstraints = true;
            }
            if (fieldAuth.minLength() != -1) {
                constraintsBuilder.setMinLength(fieldAuth.minLength());
                hasConstraints = true;
            }
            if (fieldAuth.maxLength() != -1) {
                constraintsBuilder.setMaxLength(fieldAuth.maxLength());
                hasConstraints = true;
            }
            if (!fieldAuth.regex().isEmpty()) {
                constraintsBuilder.setRegex(fieldAuth.regex());
                hasConstraints = true;
            }

            if (!fieldAuth.nullable()) {
                constraintsBuilder.setRequired(true);
                hasConstraints = true;
            }

            if (hasConstraints) {
                fieldBuilder.setConstraints(constraintsBuilder.build());
            }

        } else {
            // Default behavior
            fieldBuilder.setNullable(true);
            fieldBuilder.setOverridesOnNull(false);
        }

        FieldType fieldType = determineFieldType(reflectField.getType(), reflectField.getGenericType());
        fieldBuilder.setFieldType(fieldType);

        return fieldBuilder.build();
    }

    private static FieldType determineFieldType(Class<?> type, java.lang.reflect.Type genericType) {
        FieldType.Builder typeBuilder = FieldType.newBuilder();

        if (type == String.class || type == char.class || type == Character.class) {
            typeBuilder.setPrimitive(FieldType.Primitive.STRING);
        } else if (isNumber(type)) {
            typeBuilder.setPrimitive(FieldType.Primitive.NUMBER);
        } else if (type == boolean.class || type == Boolean.class) {
            typeBuilder.setPrimitive(FieldType.Primitive.BOOLEAN);
        } else if (type.isEnum()) {
            FieldType.Enum.Builder enumBuilder = FieldType.Enum.newBuilder();
            for (Object constant : type.getEnumConstants()) {
                enumBuilder.addVariants(constant.toString());
            }
            typeBuilder.setEnumDef(enumBuilder.build());
        } else if (Collection.class.isAssignableFrom(type) || type.isArray()) {
            // Arrays/Lists
            FieldType.Array.Builder arrayBuilder = FieldType.Array.newBuilder();
            // Determine element type
            // Note: Reflection for generic collections is tricky.
            // Simplified: If it's a list, try to get generic type.
            FieldType elementType = determineCollectionElementType(type, genericType);
            arrayBuilder.setElementType(elementType);
            typeBuilder.setArrayDef(arrayBuilder.build());
        } else {
            // Assume sub-schema (Nested Object)
            // But we can't recurse infinitely if it's the SAME class.
            // For MVP, we'll try to generate a schema for it if it has fields on it being used as data.
            // Or we treat it as unknown/String if toString()?
            // Let's recursively generate schema if it's a complex object.
            
            // NOTE: Recursive sub-schemas in this implementation might need depth control or ID references
            // to avoid cycles. For this task, we will do a simple recursion assuming value objects (DTOs).
            try {
                // If the class has @GraveyardEntity use that name, otherwise use simple name?
                // Sub-schemas in this context are embedded structures.
                String subName = type.getSimpleName();
                Schema.Builder subSchemaBuilder = Schema.newBuilder().setName(subName);

                for (java.lang.reflect.Field subReflectField : type.getDeclaredFields()) {
                    if (subReflectField.isSynthetic()
                            || Modifier.isStatic(subReflectField.getModifiers())
                            || Modifier.isTransient(subReflectField.getModifiers())) {
                        continue;
                    }

                    Field subProtoField = generateField(subReflectField);
                    subSchemaBuilder.putFields(subReflectField.getName(), subProtoField);
                }
                typeBuilder.setSubSchema(subSchemaBuilder.build());
            } catch (Exception e) {
                // Fallback to STRING? Or fail?
                throw new UnsupportedOperationException("Unsupported type for schema generation: " + type.getName(), e);
            }
        }

        return typeBuilder.build();
    }

    private static boolean isNumber(Class<?> type) {
        return Number.class.isAssignableFrom(type) 
                || type == int.class || type == long.class 
                || type == double.class || type == float.class 
                || type == short.class || type == byte.class;
    }

    private static FieldType determineCollectionElementType(Class<?> type, java.lang.reflect.Type genericType) {
        // Simplified Logic: defaulting to STRING for unknown lists
        // Proper implementation requires inspecting ParameterizedType
        if (genericType instanceof java.lang.reflect.ParameterizedType) {
            java.lang.reflect.ParameterizedType pt = (java.lang.reflect.ParameterizedType) genericType;
            java.lang.reflect.Type[] args = pt.getActualTypeArguments();
            if (args.length > 0) {
                 if (args[0] instanceof Class) {
                     return determineFieldType((Class<?>) args[0], args[0]);
                 }
            }
        }
        // Array handling
        if (type.isArray()) {
            return determineFieldType(type.getComponentType(), type.getComponentType());
        }

        return FieldType.newBuilder().setPrimitive(FieldType.Primitive.STRING).build();
    }
}
