import 'reflect-metadata';
import { Schema, Field, FieldType, FieldConstraints, FieldType_Primitive } from '../proto/eventstore';
import { ENTITY_METADATA_KEY } from '../decorators/entity';
import { FIELD_METADATA_KEY, GraveyardFieldOptions } from '../decorators/field';

export class SchemaGenerator {
    static generate(target: Function): Schema {
        return SchemaGenerator.generateInternal(target, true, new Set<Function>());
    }

    private static generateInternal(target: Function, requireEntityMetadata: boolean, visited: Set<Function>): Schema {
        if (visited.has(target)) {
            throw new Error(`Recursive schema reference detected for class ${target.name}`);
        }

        visited.add(target);
        try {
            const entityMeta = Reflect.getMetadata(ENTITY_METADATA_KEY, target);
            if (requireEntityMetadata && !entityMeta) {
                throw new Error(`Class ${target.name} is not annotated with @GraveyardEntity`);
            }

            const fieldsMeta = Reflect.getMetadata(FIELD_METADATA_KEY, target) || {};
            const schemaFields: { [key: string]: Field } = {};

            for (const [propKey, options] of Object.entries(fieldsMeta)) {
                const opts = options as GraveyardFieldOptions;
                const designType = Reflect.getMetadata("design:type", target.prototype, propKey);

                schemaFields[propKey] = {
                    fieldType: SchemaGenerator.determineFieldType(designType, propKey, visited),
                    nullable: opts.nullable ?? true,
                    overridesOnNull: opts.overridesOnNull ?? false,
                    constraints: SchemaGenerator.buildConstraints(opts)
                };
            }

            return {
                name: entityMeta?.name ?? target.name,
                fields: schemaFields
            };
        } finally {
            visited.delete(target);
        }
    }

    private static determineFieldType(type: any, propKey: string, visited: Set<Function>): FieldType {
        const fieldType: FieldType = {};

        if (type === String) {
            fieldType.primitive = FieldType_Primitive.STRING;
        } else if (type === Number) {
            fieldType.primitive = FieldType_Primitive.NUMBER;
        } else if (type === Boolean) {
            fieldType.primitive = FieldType_Primitive.BOOLEAN;
        } else if (type === Array) {
            throw new Error(
                `Field ${propKey} is an array, but the TypeScript decorator metadata cannot infer element types. ` +
                `Declare array fields explicitly in the schema or extend the generator before using them.`
            );
        } else if (type && typeof type === 'function') {
            if (type === Object || type === Date || type === Promise) {
                throw new Error(
                    `Field ${propKey} uses unsupported runtime type ${type.name}; ` +
                    `only primitive fields and decorated nested classes are supported.`
                );
            }

            const nestedFields = Reflect.getMetadata(FIELD_METADATA_KEY, type);
            if (!nestedFields && !Reflect.getMetadata(ENTITY_METADATA_KEY, type)) {
                throw new Error(
                    `Field ${propKey} uses unsupported runtime type ${type.name}; ` +
                    `only primitive fields and decorated nested classes are supported.`
                );
            }

            fieldType.subSchema = SchemaGenerator.generateInternal(type, false, visited);
        } else {
            throw new Error(
                `Field ${propKey} has no usable runtime type metadata. ` +
                `Ensure emitDecoratorMetadata is enabled and only supported field types are used.`
            );
        }

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
