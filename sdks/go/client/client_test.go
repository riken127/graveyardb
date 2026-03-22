package client

import (
	"context"
	"strings"
	"testing"

	"google.golang.org/grpc/metadata"

	pb "github.com/riken127/graveyar_db/sdks/go/proto"
)

func TestEncodeExpectedVersion(t *testing.T) {
	got, err := encodeExpectedVersion(ExpectedVersionAny)
	if err != nil {
		t.Fatalf("encodeExpectedVersion failed: %v", err)
	}
	if got != ExpectedVersionAny {
		t.Fatalf("expected sentinel to stay as -1, got %d", got)
	}

	got, err = encodeExpectedVersion(42)
	if err != nil {
		t.Fatalf("encodeExpectedVersion failed: %v", err)
	}
	if got != 42 {
		t.Fatalf("expected 42, got %d", got)
	}

	if _, err := encodeExpectedVersion(ExpectedVersionAny - 1); err == nil {
		t.Fatalf("expected negative versions below the sentinel to fail")
	} else if err.Error() != "expected_version must be -1 or a non-negative version" {
		t.Fatalf("unexpected error message %q", err)
	}
}

func TestValidateTransition(t *testing.T) {
	if err := validateTransition(&pb.Transition{
		Name:      "Activated",
		FromState: "pending",
		ToState:   "active",
	}, 0); err != nil {
		t.Fatalf("validateTransition failed: %v", err)
	}

	if err := validateTransition(nil, 1); err == nil {
		t.Fatalf("expected missing transition to fail")
	}

	if err := validateTransition(&pb.Transition{Name: " ", FromState: "pending", ToState: "active"}, 2); err == nil || !strings.Contains(err.Error(), "transition.name") {
		t.Fatalf("expected blank transition name to fail, got %v", err)
	}

	if err := validateTransition(&pb.Transition{Name: "Activated", FromState: "active", ToState: "active"}, 3); err == nil || !strings.Contains(err.Error(), "must be different") {
		t.Fatalf("expected identical states to fail, got %v", err)
	}
}

func TestValidateAppendEvents(t *testing.T) {
	if err := validateAppendEvents([]*pb.Event{
		{
			Id:        "1",
			EventType: "Created",
			Transition: &pb.Transition{
				Name:      "Created",
				FromState: "draft",
				ToState:   "published",
			},
		},
	}); err != nil {
		t.Fatalf("validateAppendEvents failed: %v", err)
	}

	if err := validateAppendEvents([]*pb.Event{{}}); err == nil {
		t.Fatalf("expected missing transition to fail")
	}
}

func TestWithAuthAddsMetadata(t *testing.T) {
	c := &Client{
		config: Config{AuthToken: "secret-token"},
	}

	ctx := c.withAuth(context.Background())
	md, ok := metadata.FromOutgoingContext(ctx)
	if !ok {
		t.Fatalf("expected outgoing metadata")
	}

	values := md.Get("authorization")
	if len(values) != 1 {
		t.Fatalf("expected one authorization header, got %v", values)
	}
	if values[0] != "Bearer secret-token" {
		t.Fatalf("unexpected authorization header %q", values[0])
	}
}

func TestTransportCredentialsWrapsCAFileErrors(t *testing.T) {
	_, err := transportCredentials(Config{
		UseTLS:      true,
		TLSCertFile: "/definitely/not/present.pem",
	})
	if err == nil {
		t.Fatalf("expected missing CA file to fail")
	}
	if got := err.Error(); got == "" || !strings.Contains(got, "/definitely/not/present.pem") {
		t.Fatalf("expected error to mention the CA file path, got %q", got)
	}
}

func TestTransportCredentialsDefaultTLS(t *testing.T) {
	if _, err := transportCredentials(Config{UseTLS: true}); err != nil {
		t.Fatalf("transportCredentials failed: %v", err)
	}
}
