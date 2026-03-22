package client

import "time"

// Config holds the configuration for the EventStore client.
type Config struct {
	// Address is the target address of the EventStore server (e.g., "localhost:50051").
	Address string

	// Timeout is the default timeout for gRPC calls.
	// If zero, no timeout is applied by default (though context deadline still applies).
	Timeout time.Duration

	// UseTLS indicates whether to use a secure TLS connection.
	UseTLS bool

	// TLSCertFile is the path to the CA certificate file for verifying the server's certificate.
	// If empty and UseTLS is true, the system's root CAs will be used.
	TLSCertFile string

	// AuthToken is sent as a Bearer token on outgoing gRPC requests when set.
	AuthToken string
}

// DefaultConfig returns a default configuration with:
// Address: "localhost:50051"
// Timeout: 5 seconds
// UseTLS: false
func DefaultConfig() Config {
	return Config{
		Address: "localhost:50051",
		Timeout: 5 * time.Second,
		UseTLS:  false,
	}
}

// NewDefaultConfig is kept for backwards-compatible docs and examples.
func NewDefaultConfig() Config {
	return DefaultConfig()
}
