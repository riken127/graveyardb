package client

import (
	"context"
	"crypto/tls"
	"fmt"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials"
	"google.golang.org/grpc/credentials/insecure"
	"google.golang.org/grpc/metadata"

	pb "github.com/riken127/graveyar_db/sdks/go/proto"
	"github.com/riken127/graveyar_db/sdks/go/schema"
)

// ExpectedVersionAny disables optimistic concurrency checks for an append.
const ExpectedVersionAny int64 = -1

// GenerateSchema is a helper to generate a Schema definition from a Go struct.
func GenerateSchema(v interface{}) (*pb.Schema, error) {
	return schema.Generate(v)
}

// Client is the high-level client for interacting with the graveyar_db Event Store.
// It manages the underlying gRPC connection and provides strongly-typed methods
// for appending and reading events.
type Client struct {
	conn   *grpc.ClientConn
	client pb.EventStoreClient
	config Config
}

// NewClient creates a new Client with the provided configuration.
// It establishes a gRPC connection to the server specified in config.Address.
func NewClient(config Config) (*Client, error) {
	var opts []grpc.DialOption

	if config.UseTLS {
		var creds credentials.TransportCredentials
		var err error

		if config.TLSCertFile != "" {
			creds, err = credentials.NewClientTLSFromFile(config.TLSCertFile, "")
			if err != nil {
				return nil, err
			}
		} else {
			// Use system root CAs
			// Note: This requires "crypto/x509" and "google.golang.org/grpc/credentials"
			// Since we want to keep imports clean, we'll try standard loading if needed
			// actually credentials.NewTLS(nil) uses system roots
			creds = credentials.NewTLS(&tls.Config{})
		}
		opts = append(opts, grpc.WithTransportCredentials(creds))
	} else {
		opts = append(opts, grpc.WithTransportCredentials(insecure.NewCredentials()))
	}

	conn, err := grpc.Dial(config.Address, opts...)
	if err != nil {
		return nil, err
	}
	return &Client{
		conn:   conn,
		client: pb.NewEventStoreClient(conn),
		config: config,
	}, nil
}

// Close closes the underlying gRPC connection.
// It should be called when the client is no longer needed.
func (c *Client) Close() error {
	return c.conn.Close()
}

func (c *Client) unaryContext(ctx context.Context) (context.Context, context.CancelFunc) {
	ctx = c.withAuth(ctx)
	if _, ok := ctx.Deadline(); !ok && c.config.Timeout > 0 {
		return context.WithTimeout(ctx, c.config.Timeout)
	}

	return ctx, func() {}
}

func (c *Client) withAuth(ctx context.Context) context.Context {
	if c.config.AuthToken == "" {
		return ctx
	}

	return metadata.AppendToOutgoingContext(ctx, "authorization", "Bearer "+c.config.AuthToken)
}

func encodeExpectedVersion(expectedVersion int64) (int64, error) {
	switch {
	case expectedVersion < ExpectedVersionAny:
		return 0, fmt.Errorf("expectedVersion must be %d or greater", ExpectedVersionAny)
	default:
		return expectedVersion, nil
	}
}

// AppendEvent appends a batch of events to a specific stream.
//
// streamID: The unique identifier of the stream.
// events: The list of events to append.
// expectedVersion: Optimistic locking version.
// Use client.ExpectedVersionAny to disable version checking, or pass a specific
// version number (0, 1, ...) to enforce strict ordering.
//
// Returns true if the append was successful, or an error if the RPC failed
// or the version check failed on the server side (depending on server error implementation).
func (c *Client) AppendEvent(ctx context.Context, streamID string, events []*pb.Event, expectedVersion int64) (bool, error) {
	ctx, cancel := c.unaryContext(ctx)
	defer cancel()

	encodedExpectedVersion, err := encodeExpectedVersion(expectedVersion)
	if err != nil {
		return false, err
	}

	req := &pb.AppendEventRequest{
		StreamId:        streamID,
		Events:          events,
		ExpectedVersion: encodedExpectedVersion,
	}

	resp, err := c.client.AppendEvent(ctx, req)
	if err != nil {
		return false, err
	}
	return resp.Success, nil
}

// GetEvents opens a stream to read events from the specified streamID.
// It returns a gRPC stream client that can be used to receive events.
func (c *Client) GetEvents(ctx context.Context, streamID string) (pb.EventStore_GetEventsClient, error) {
	ctx = c.withAuth(ctx)

	req := &pb.GetEventsRequest{
		StreamId: streamID,
	}
	return c.client.GetEvents(ctx, req)
}

// UpsertSchema registers or updates a schema definition.
func (c *Client) UpsertSchema(ctx context.Context, schema *pb.Schema) (*pb.UpsertSchemaResponse, error) {
	ctx, cancel := c.unaryContext(ctx)
	defer cancel()

	req := &pb.UpsertSchemaRequest{
		Schema: schema,
	}
	return c.client.UpsertSchema(ctx, req)
}

// GetSchema retrieves a schema definition by name.
func (c *Client) GetSchema(ctx context.Context, name string) (*pb.GetSchemaResponse, error) {
	ctx, cancel := c.unaryContext(ctx)
	defer cancel()

	req := &pb.GetSchemaRequest{
		Name: name,
	}
	return c.client.GetSchema(ctx, req)
}
