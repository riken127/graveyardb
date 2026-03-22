package schema

import (
	"encoding/json"
	"testing"

	pb "github.com/riken127/graveyar_db/sdks/go/proto"
)

type ValidationStruct struct {
	Name    string `json:"full_name"`
	Age     int
	Active  bool
	Tags    []string
	Address *AddressStruct
}

type AddressStruct struct {
	Street string
	City   string
}

type TaggedStruct struct {
	Username string `json:"username" graveyard:"min_length=3,max_length=12,regex=^[a-z]+$,required"`
	Age      *int   `graveyard:"min=18,max=150,nullable=true"`
}

func TestGenerate(t *testing.T) {
	v := ValidationStruct{}
	schema, err := Generate(v)
	if err != nil {
		t.Fatalf("Generate failed: %v", err)
	}

	// Basic check
	if schema.Name != "ValidationStruct" {
		t.Errorf("Expected schema name ValidationStruct, got %s", schema.Name)
	}

	// Check fields
	if len(schema.Fields) != 5 {
		t.Errorf("Expected 5 fields, got %d", len(schema.Fields))
	}

	// name field (mapped from json tag)
	if f, ok := schema.Fields["full_name"]; !ok {
		t.Errorf("Missing full_name field")
	} else {
		// Check type
		if _, ok := f.FieldType.Kind.(*pb.FieldType_Primitive_); !ok {
			t.Errorf("full_name should be primitive")
		}
	}

	// Address field (SubSchema + Nullable)
	if f, ok := schema.Fields["Address"]; !ok {
		t.Errorf("Missing Address field")
	} else {
		if !f.Nullable {
			t.Errorf("Address should be nullable pointer")
		}
		if _, ok := f.FieldType.Kind.(*pb.FieldType_SubSchema); !ok {
			t.Errorf("Address should be SubSchema")
		}
	}

	// Tags field (Array)
	if f, ok := schema.Fields["Tags"]; !ok {
		t.Errorf("Missing Tags field")
	} else {
		if arr, ok := f.FieldType.Kind.(*pb.FieldType_ArrayDef); !ok {
			t.Errorf("Tags should be Array")
		} else {
			// Check element type
			if _, ok := arr.ArrayDef.ElementType.Kind.(*pb.FieldType_Primitive_); !ok {
				t.Errorf("Tags element type should be primitive")
			}
		}
	}

	// Debug print
	b, _ := json.MarshalIndent(schema, "", "  ")
	t.Logf("Generated Schema: %s", string(b))
}

func TestGenerateParsesGraveyardTags(t *testing.T) {
	schema, err := Generate(TaggedStruct{})
	if err != nil {
		t.Fatalf("Generate failed: %v", err)
	}

	username := schema.Fields["username"]
	if username == nil {
		t.Fatalf("missing username field")
	}
	if username.Nullable {
		t.Fatalf("username should be required by default")
	}
	if username.Constraints == nil || !username.Constraints.Required {
		t.Fatalf("username should be required")
	}
	if got := username.Constraints.MinLength; got == nil || *got != 3 {
		t.Fatalf("expected min_length=3, got %v", got)
	}
	if got := username.Constraints.MaxLength; got == nil || *got != 12 {
		t.Fatalf("expected max_length=12, got %v", got)
	}
	if got := username.Constraints.Regex; got == nil || *got != "^[a-z]+$" {
		t.Fatalf("expected regex to be parsed, got %v", got)
	}

	age := schema.Fields["Age"]
	if age == nil {
		t.Fatalf("missing Age field")
	}
	if !age.Nullable {
		t.Fatalf("Age should be nullable because the tag says so")
	}
	if age.Constraints == nil || age.Constraints.Required {
		t.Fatalf("Age should not be required")
	}
	if got := age.Constraints.MinValue; got == nil || *got != 18 {
		t.Fatalf("expected min=18, got %v", got)
	}
	if got := age.Constraints.MaxValue; got == nil || *got != 150 {
		t.Fatalf("expected max=150, got %v", got)
	}
}
