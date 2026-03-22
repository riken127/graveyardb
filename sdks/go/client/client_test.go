package client

import (
	"context"
	"testing"

	"google.golang.org/grpc/metadata"
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
