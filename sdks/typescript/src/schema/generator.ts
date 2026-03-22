import 'reflect-metadata';
import { Schema, Field, FieldType, FieldConstraints, FieldType_Primitive } from '../proto/eventstore';
import { ENTITY_METADATA_KEY } from '../decorators/entity';
import { FIELD_METADATA_KEY, GraveyardFieldOptions } from '../decorators/field';

export class SchemaGenerator {
    static generate(target: Function): Schema {
        const entityMeta = Reflect.getMetadata(ENTITY_METADATA_KEY, target);
        if (!entityMeta) {
            throw new Error(`Class ${target.name} is not annotated with @GraveyardEntity`);
        }

        const fieldsMeta = Reflect.getMetadata(FIELD_METADATA_KEY, target) || {};
        const schemaFields: { [key: string]: Field } = {};

        // In TS, we don't have easy reflection for types at runtime without emitDecoratorMetadata
        // and even then it's limited (Object, String, Number).
        // We will infer basics from 'design:type' metadata if available, or rely on manual spec/convention?
        // With 'emitDecoratorMetadata: true', we get basic types.

        for (const [propKey, options] of Object.entries(fieldsMeta)) {
            const opts = options as GraveyardFieldOptions;
            const designType = Reflect.getMetadata("design:type", target.prototype, propKey);

            schemaFields[propKey] = {
                fieldType: SchemaGenerator.determineFieldType(designType),
                nullable: opts.nullable ?? true,
                overridesOnNull: opts.overridesOnNull ?? false,
                constraints: SchemaGenerator.buildConstraints(opts)
            };
        }

        return {
            name: entityMeta.name,
            fields: schemaFields
        };
    }

    private static determineFieldType(type: any): FieldType {
        const fieldType: FieldType = {};

        if (type === String) {
            fieldType.primitive = FieldType_Primitive.STRING;
        } else if (type === Number) {
            fieldType.primitive = FieldType_Primitive.NUMBER;
        } else if (type === Boolean) {
            fieldType.primitive = FieldType_Primitive.BOOLEAN;
        } else if (type === Array) {
            // Arrays are hard to infer element type from without manual specification in decorator
            // For MVP, defaulting to array of Strings or we need explicit type in decorator
            fieldType.arrayDef = {
                elementType: { primitive: FieldType_Primitive.STRING }
            };
        } else {
            // Default to STRING if unknown
            fieldType.primitive = FieldType_Primitive.STRING;
        }
        // TODO: Handle nested schemas, Enums

        return fieldType;
    }

    private static buildConstraints(opts: GraveyardFieldOptions): FieldConstraints | undefined {
        let hasConstraints = false;
        const c: FieldConstraints = {
            required: opts.nullable === false
        };

        if (opts.min !== undefined) { c.minValue = opts.min; hasConstraints = true; }
        if (opts.max !== undefined) { c.maxValue = opts.max; hasConstraints = true; }
        if (opts.minLength !== undefined) { c.minLength = opts.minLength; hasConstraints = true; }
        if (opts.maxLength !== undefined) { c.maxLength = opts.maxLength; hasConstraints = true; }
        if (opts.regex !== undefined) { c.regex = opts.regex; hasConstraints = true; }

        return hasConstraints || c.required ? c : undefined;
    }
}
