# EventStore Go SDK

Go client library for `graveyar_db` using gRPC.

## Features

- `DefaultConfig()` with sensible defaults for local development.
- `ExpectedVersionAny` for appends that should skip optimistic concurrency checks.
- Bearer token propagation via `Config.AuthToken`.
- Struct-to-schema generation with `json` field renaming and `graveyard` constraints.
- TLS and per-request timeout support.

## Installation

```bash
go get github.com/riken127/graveyar_db/sdks/go
```

## Usage

The snippets below assume these imports and a request context:

```go
import (
	"context"
	"io"
	"log"
	"os"
	"time"

	"github.com/riken127/graveyar_db/sdks/go/client"
	pb "github.com/riken127/graveyar_db/sdks/go/proto"
)

ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
defer cancel()
```

### Client Setup

```go
cfg := client.DefaultConfig()
cfg.Address = "localhost:50051"
cfg.AuthToken = os.Getenv("EVENTSTORE_AUTH_TOKEN")

c, err := client.NewClient(cfg)
if err != nil {
	log.Fatal(err)
}
defer c.Close()
```

### Append Events

```go
events := []*pb.Event{
	{
		Id:        "123",
		EventType: "UserCreated",
		Payload:   []byte(`{"name":"Ada"}`),
		Timestamp: uint64(time.Now().UnixMilli()),
		Transition: &pb.Transition{
			Name:      "UserCreated",
			FromState: "draft",
			ToState:   "active",
		},
	},
}

success, err := c.AppendEvent(ctx, "user-123", events, client.ExpectedVersionAny)
if err != nil {
	log.Fatal(err)
}
```

Use `client.ExpectedVersionAny` when you want the server to accept the append without checking the current stream version. Pass an explicit version when you want optimistic concurrency control.
Only `-1` and non-negative versions are valid; the client rejects smaller values before the RPC is sent.
Each event must also include a non-empty `transition` with `name`, `from_state`, and `to_state`, and `from_state` must differ from `to_state`.

### Generate and Register a Schema

```go
type User struct {
	Name string `json:"full_name" graveyard:"required,min_length=3"`
	Age  int    `graveyard:"min=18"`
}

schema, err := client.GenerateSchema(User{})
if err != nil {
	log.Fatal(err)
}

_, err = c.UpsertSchema(ctx, schema)
if err != nil {
	log.Fatal(err)
}
```

### Read Events

`GetEvents` returns a streaming gRPC client. Read with `Recv()` until `io.EOF`.

```go
stream, err := c.GetEvents(ctx, "user-123")
if err != nil {
	log.Fatal(err)
}

for {
	event, err := stream.Recv()
	if err == io.EOF {
		break
	}
	if err != nil {
		log.Fatal(err)
	}
	log.Printf("event %s", event.GetId())
}
```

## Notes

- `Config.Timeout` applies to unary RPCs.
- `Config.AuthToken` is sent as `authorization: Bearer <token>` on outgoing unary and streaming gRPC requests.
- `Config.TLSCertFile` should point at a CA bundle used to verify the server certificate; when it is empty, Go's system root store is used.
- The generated schema uses exported Go field names unless you provide a `json` tag. `json:"-"` omits a field from the schema entirely.
- Schema generation does not currently support raw `[]byte` fields because the proto schema model has no byte primitive.
