package com.eventstore.client.validation;

import com.eventstore.client.model.Field;
import com.eventstore.client.model.FieldType;
import com.eventstore.client.model.Schema;
import org.junit.jupiter.api.Test;

import java.nio.charset.StandardCharsets;
import java.util.List;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

class SchemaValidatorTest {

    @Test
    void rejectsMissingValueForNonNullableFieldWithoutExplicitConstraints() {
        Schema schema = Schema.newBuilder()
                .setName("user")
                .putFields("username", Field.newBuilder()
                        .setNullable(false)
                        .setFieldType(FieldType.newBuilder().setPrimitive(FieldType.Primitive.STRING).build())
                        .build())
                .build();

        List<String> errors = SchemaValidator.validate("{}".getBytes(StandardCharsets.UTF_8), schema);

        assertEquals(1, errors.size());
        assertTrue(errors.get(0).contains("username"));
    }

    @Test
    void rejectsExplicitNullForNonNullableFieldWithoutExplicitConstraints() {
        Schema schema = Schema.newBuilder()
                .setName("user")
                .putFields("username", Field.newBuilder()
                        .setNullable(false)
                        .setFieldType(FieldType.newBuilder().setPrimitive(FieldType.Primitive.STRING).build())
                        .build())
                .build();

        List<String> errors = SchemaValidator.validate("{\"username\":null}".getBytes(StandardCharsets.UTF_8), schema);

        assertEquals(1, errors.size());
        assertTrue(errors.get(0).contains("username"));
    }
}
