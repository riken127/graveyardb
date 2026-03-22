# SDK Matrix

This table summarizes the current client SDK coverage in this repository.

| Capability | Go | Java | TypeScript | Notes |
| --- | --- | --- | --- | --- |
| Append events | Yes | Yes | Yes | All three validate transition presence and state movement before sending the RPC. |
| Read events | Yes | Yes | Yes | Go returns a streaming gRPC client, Java returns an iterator, and TypeScript buffers the full result set into memory. |
| Upsert schema | Yes | Yes | Yes | Go uses struct tags, Java uses annotations, and TypeScript uses decorators. |
| Save snapshot | No | Yes | No | Only the Java SDK currently exposes snapshot RPC wrappers. |
| Get snapshot | No | Yes | No | Same as above. |
| `ANY_VERSION` / `ExpectedVersionAny` | Yes | Yes | Yes | The sentinel value is `-1`. |
| TLS support | Yes | Yes | Yes | TLS is configurable per SDK, but the implementation details differ by language. |
| Bearer token auth | Yes | Yes | Yes | SDKs send `authorization: Bearer <token>` when configured. |
| Client-side timeout | Yes | Yes | Yes | Go uses `context.WithTimeout`, Java uses gRPC deadlines, and TypeScript uses request deadlines. |
| Schema generation | Yes | Yes | Yes | TypeScript still rejects array inference in the generator; Go and Java document their own modeling limits. |

## Notes

* Java currently offers the broadest RPC surface because it exposes the snapshot methods in addition to append, read, and schema operations.
* TypeScript currently provides a buffered `getEvents` convenience API instead of a live stream iterator.
* Use the release checklist to confirm the exact behavior you depend on before production use.
* All SDKs should be validated against the current server release before you depend on them in production.
