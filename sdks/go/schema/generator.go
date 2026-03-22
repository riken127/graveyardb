package schema

import (
	"fmt"
	"reflect"
	"strconv"
	"strings"

	pb "github.com/riken127/graveyar_db/sdks/go/proto"
)

type graveyardTagOptions struct {
	nullableSet        bool
	nullable           bool
	overridesOnNullSet bool
	overridesOnNull    bool
	requiredSet        bool
	required           bool
	minSet             bool
	min                float64
	maxSet             bool
	max                float64
	minLengthSet       bool
	minLength          int32
	maxLengthSet       bool
	maxLength          int32
	regexSet           bool
	regex              string
}

// Generate creates a protobuf Schema definition from a Go struct.
func Generate(v interface{}) (*pb.Schema, error) {
	if v == nil {
		return nil, fmt.Errorf("schema generation requires a non-nil struct value")
	}

	t := reflect.TypeOf(v)
	// Dereference pointer if needed
	if t.Kind() == reflect.Ptr {
		t = t.Elem()
	}

	if t.Kind() != reflect.Struct {
		return nil, fmt.Errorf("schema generation requires a struct, got %s", t.Kind())
	}

	return generateStructSchema(t)
}

func generateStructSchema(t reflect.Type) (*pb.Schema, error) {
	schemaName := t.Name()
	if schemaName == "" {
		return nil, fmt.Errorf("schema generation requires a named struct type")
	}

	fields := make(map[string]*pb.Field)

	for i := 0; i < t.NumField(); i++ {
		f := t.Field(i)

		// Skip unexported fields
		if f.PkgPath != "" {
			continue
		}

		fieldName := f.Name

		jsonTag := f.Tag.Get("json")

		if jsonTag == "-" {
			continue
		}

		if jsonTag != "" {
			parts := strings.Split(jsonTag, ",")
			if parts[0] != "" {
				fieldName = parts[0]
			}
		}

		tagOptions, err := parseGraveyardTag(f.Tag.Get("graveyard"))
		if err != nil {
			return nil, fmt.Errorf("field %s: %w", f.Name, err)
		}

		pbField, err := mapField(f.Type, tagOptions)
		if err != nil {
			return nil, fmt.Errorf("field %s: %w", f.Name, err)
		}

		fields[fieldName] = pbField
	}

	return &pb.Schema{
		Name:   schemaName,
		Fields: fields,
	}, nil
}

// mapField handles the creation of a Field, including processing nullability/pointers
func mapField(t reflect.Type, opts graveyardTagOptions) (*pb.Field, error) {
	nullable := false
	currentType := t

	if currentType.Kind() == reflect.Ptr {
		nullable = true
		currentType = currentType.Elem()
	}

	if opts.nullableSet {
		nullable = opts.nullable
	}

	fieldType, err := mapFieldType(currentType)
	if err != nil {
		return nil, err
	}

	required := !nullable
	if opts.requiredSet {
		required = opts.required
	}

	constraints := buildConstraints(opts, required)

	return &pb.Field{
		FieldType:       fieldType,
		Nullable:        nullable,
		OverridesOnNull: opts.overridesOnNull,
		Constraints:     constraints,
	}, nil
}

// mapFieldType handles the creation of the FieldType definitions (recursive)
func mapFieldType(t reflect.Type) (*pb.FieldType, error) {
	ft := &pb.FieldType{}

	switch t.Kind() {
	case reflect.String:
		ft.Kind = &pb.FieldType_Primitive_{Primitive: pb.FieldType_STRING}
	case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64,
		reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64,
		reflect.Float32, reflect.Float64:
		ft.Kind = &pb.FieldType_Primitive_{Primitive: pb.FieldType_NUMBER}
	case reflect.Bool:
		ft.Kind = &pb.FieldType_Primitive_{Primitive: pb.FieldType_BOOLEAN}
	case reflect.Slice, reflect.Array:
		if t.Elem().Kind() == reflect.Uint8 {
			return nil, fmt.Errorf("[]byte is not supported by the schema proto; use string or a nested schema field instead")
		}

		elemType, err := mapFieldType(t.Elem())
		if err != nil {
			return nil, err
		}
		ft.Kind = &pb.FieldType_ArrayDef{ArrayDef: &pb.FieldType_Array{ElementType: elemType}}

	case reflect.Struct:
		subSchema, err := generateStructSchema(t)
		if err != nil {
			return nil, err
		}
		ft.Kind = &pb.FieldType_SubSchema{SubSchema: subSchema}

	default:
		return nil, fmt.Errorf("unsupported type: %s", t.String())
	}

	return ft, nil
}

func buildConstraints(opts graveyardTagOptions, required bool) *pb.FieldConstraints {
	constraints := &pb.FieldConstraints{
		Required: required,
	}
	hasConstraint := opts.requiredSet

	if opts.minSet {
		constraints.MinValue = &opts.min
		hasConstraint = true
	}
	if opts.maxSet {
		constraints.MaxValue = &opts.max
		hasConstraint = true
	}
	if opts.minLengthSet {
		constraints.MinLength = &opts.minLength
		hasConstraint = true
	}
	if opts.maxLengthSet {
		constraints.MaxLength = &opts.maxLength
		hasConstraint = true
	}
	if opts.regexSet {
		constraints.Regex = &opts.regex
		hasConstraint = true
	}

	if !hasConstraint && !required {
		return nil
	}

	return constraints
}

func parseGraveyardTag(tag string) (graveyardTagOptions, error) {
	if tag == "" {
		return graveyardTagOptions{}, nil
	}

	opts := graveyardTagOptions{}
	parts := strings.Split(tag, ",")
	for _, rawPart := range parts {
		part := strings.TrimSpace(rawPart)
		if part == "" {
			continue
		}

		key, value, hasValue := strings.Cut(part, "=")
		key = strings.ToLower(strings.TrimSpace(key))
		value = strings.TrimSpace(value)

		if !hasValue {
			switch key {
			case "required":
				opts.requiredSet = true
				opts.required = true
			case "nullable":
				opts.nullableSet = true
				opts.nullable = true
			case "overrides_on_null":
				opts.overridesOnNullSet = true
				opts.overridesOnNull = true
			default:
				return graveyardTagOptions{}, fmt.Errorf("unsupported graveyard tag option %q", part)
			}
			continue
		}

		switch key {
		case "nullable":
			b, err := strconv.ParseBool(value)
			if err != nil {
				return graveyardTagOptions{}, fmt.Errorf("invalid nullable value %q: %w", value, err)
			}
			opts.nullableSet = true
			opts.nullable = b
		case "required":
			b, err := strconv.ParseBool(value)
			if err != nil {
				return graveyardTagOptions{}, fmt.Errorf("invalid required value %q: %w", value, err)
			}
			opts.requiredSet = true
			opts.required = b
		case "overrides_on_null":
			b, err := strconv.ParseBool(value)
			if err != nil {
				return graveyardTagOptions{}, fmt.Errorf("invalid overrides_on_null value %q: %w", value, err)
			}
			opts.overridesOnNullSet = true
			opts.overridesOnNull = b
		case "min":
			v, err := strconv.ParseFloat(value, 64)
			if err != nil {
				return graveyardTagOptions{}, fmt.Errorf("invalid min value %q: %w", value, err)
			}
			opts.minSet = true
			opts.min = v
		case "max":
			v, err := strconv.ParseFloat(value, 64)
			if err != nil {
				return graveyardTagOptions{}, fmt.Errorf("invalid max value %q: %w", value, err)
			}
			opts.maxSet = true
			opts.max = v
		case "min_length", "min_len":
			v, err := strconv.ParseInt(value, 10, 32)
			if err != nil {
				return graveyardTagOptions{}, fmt.Errorf("invalid min_length value %q: %w", value, err)
			}
			opts.minLengthSet = true
			opts.minLength = int32(v)
		case "max_length", "max_len":
			v, err := strconv.ParseInt(value, 10, 32)
			if err != nil {
				return graveyardTagOptions{}, fmt.Errorf("invalid max_length value %q: %w", value, err)
			}
			opts.maxLengthSet = true
			opts.maxLength = int32(v)
		case "regex":
			opts.regexSet = true
			opts.regex = value
		default:
			return graveyardTagOptions{}, fmt.Errorf("unsupported graveyard tag option %q", part)
		}
	}

	return opts, nil
}
