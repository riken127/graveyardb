package client

import (
	"context"
	"strings"
	"testing"

	"google.golang.org/grpc"
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

type mockEventStoreClient struct {
	appendEventRequest   *pb.AppendEventRequest
	appendEventResponse  *pb.AppendEventResponse
	getSchemaRequest     *pb.GetSchemaRequest
	getSchemaResponse    *pb.GetSchemaResponse
	saveSnapshotRequest  *pb.SaveSnapshotRequest
	saveSnapshotResponse *pb.SaveSnapshotResponse
	getSnapshotRequest   *pb.GetSnapshotRequest
	getSnapshotResponse  *pb.GetSnapshotResponse
}

func (m *mockEventStoreClient) AppendEvent(ctx context.Context, in *pb.AppendEventRequest, opts ...grpc.CallOption) (*pb.AppendEventResponse, error) {
	m.appendEventRequest = in
	if m.appendEventResponse == nil {
		m.appendEventResponse = &pb.AppendEventResponse{}
	}
	return m.appendEventResponse, nil
}

func (m *mockEventStoreClient) GetEvents(context.Context, *pb.GetEventsRequest, ...grpc.CallOption) (grpc.ServerStreamingClient[pb.Event], error) {
	return nil, nil
}

func (m *mockEventStoreClient) UpsertSchema(context.Context, *pb.UpsertSchemaRequest, ...grpc.CallOption) (*pb.UpsertSchemaResponse, error) {
	return &pb.UpsertSchemaResponse{}, nil
}

func (m *mockEventStoreClient) GetSchema(ctx context.Context, in *pb.GetSchemaRequest, opts ...grpc.CallOption) (*pb.GetSchemaResponse, error) {
	m.getSchemaRequest = in
	if m.getSchemaResponse == nil {
		m.getSchemaResponse = &pb.GetSchemaResponse{}
	}
	return m.getSchemaResponse, nil
}

func (m *mockEventStoreClient) SaveSnapshot(ctx context.Context, in *pb.SaveSnapshotRequest, opts ...grpc.CallOption) (*pb.SaveSnapshotResponse, error) {
	m.saveSnapshotRequest = in
	if m.saveSnapshotResponse == nil {
		m.saveSnapshotResponse = &pb.SaveSnapshotResponse{}
	}
	return m.saveSnapshotResponse, nil
}

func (m *mockEventStoreClient) GetSnapshot(ctx context.Context, in *pb.GetSnapshotRequest, opts ...grpc.CallOption) (*pb.GetSnapshotResponse, error) {
	m.getSnapshotRequest = in
	if m.getSnapshotResponse == nil {
		m.getSnapshotResponse = &pb.GetSnapshotResponse{}
	}
	return m.getSnapshotResponse, nil
}

func TestGetSchema(t *testing.T) {
	mock := &mockEventStoreClient{
		getSchemaResponse: &pb.GetSchemaResponse{Found: true, Schema: &pb.Schema{Name: "user"}},
	}
	c := &Client{client: mock, config: Config{Timeout: 5_000_000_000}}

	resp, err := c.GetSchema(context.Background(), "user")
	if err != nil {
		t.Fatalf("GetSchema failed: %v", err)
	}
	if resp == nil || !resp.GetFound() {
		t.Fatalf("expected schema response, got %#v", resp)
	}
	if mock.getSchemaRequest == nil {
		t.Fatalf("expected request to be recorded")
	}
	if got := mock.getSchemaRequest.GetName(); got != "user" {
		t.Fatalf("expected schema name %q, got %q", "user", got)
	}
}

func TestSaveSnapshot(t *testing.T) {
	mock := &mockEventStoreClient{
		saveSnapshotResponse: &pb.SaveSnapshotResponse{Success: true},
	}
	c := &Client{client: mock, config: Config{Timeout: 5_000_000_000}}
	snapshot := &pb.Snapshot{
		StreamId:  "stream-1",
		Version:   7,
		Payload:   []byte("payload"),
		Timestamp: 1234,
	}

	saved, err := c.SaveSnapshot(context.Background(), snapshot)
	if err != nil {
		t.Fatalf("SaveSnapshot failed: %v", err)
	}
	if !saved {
		t.Fatalf("expected snapshot save to succeed")
	}
	if mock.saveSnapshotRequest == nil {
		t.Fatalf("expected request to be recorded")
	}
	if got := mock.saveSnapshotRequest.GetSnapshot(); got == nil || got.GetStreamId() != "stream-1" || got.GetVersion() != 7 {
		t.Fatalf("unexpected snapshot request: %#v", got)
	}
}

func TestGetSnapshot(t *testing.T) {
	mock := &mockEventStoreClient{
		getSnapshotResponse: &pb.GetSnapshotResponse{
			Found: true,
			Snapshot: &pb.Snapshot{
				StreamId:  "stream-2",
				Version:   11,
				Timestamp: 99,
			},
		},
	}
	c := &Client{client: mock, config: Config{Timeout: 5_000_000_000}}

	snapshot, err := c.GetSnapshot(context.Background(), "stream-2")
	if err != nil {
		t.Fatalf("GetSnapshot failed: %v", err)
	}
	if snapshot == nil || snapshot.GetStreamId() != "stream-2" || snapshot.GetVersion() != 11 {
		t.Fatalf("unexpected snapshot result: %#v", snapshot)
	}
	if mock.getSnapshotRequest == nil {
		t.Fatalf("expected request to be recorded")
	}
	if got := mock.getSnapshotRequest.GetStreamId(); got != "stream-2" {
		t.Fatalf("expected stream id %q, got %q", "stream-2", got)
	}
}

func TestGetSnapshot_NotFound(t *testing.T) {
	mock := &mockEventStoreClient{
		getSnapshotResponse: &pb.GetSnapshotResponse{Found: false},
	}
	c := &Client{client: mock, config: Config{Timeout: 5_000_000_000}}

	snapshot, err := c.GetSnapshot(context.Background(), "missing")
	if err != nil {
		t.Fatalf("GetSnapshot failed: %v", err)
	}
	if snapshot != nil {
		t.Fatalf("expected nil snapshot when not found, got %#v", snapshot)
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
