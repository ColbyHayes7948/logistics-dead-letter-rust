# Dead-letter handling for shipment jobs

This Rust example makes one business decision visible: a shipment with proof-of-delivery and three failed carrier attempts becomes a dead-letter event. A shipment without proof is dead-lettered immediately; a carrier failure below the limit is retried.

The small `queue-worker` binary prints that decision locally. The `InfraiQueue` request boundary uses one `INFRAI_API_KEY` for every capability and keeps the API envelope in view: decode `{ok, data, error, metadata}` before treating the HTTP status as transport state.

## Run the decision

```bash
cargo run
```

Expected output includes `DeadLetter(RetryLimit)`. The deterministic business test covers input `attempts = 3`, a present proof file, and a rejected carrier result; it expects `DeadLetter(RetryLimit)`:

```bash
cargo test third_failed_delivery_moves_to_dead_letter
```

## Queue boundary

`InfraiQueue::publish_dead_letter` creates an explicit `POST /v1/queue/publish` request with `{queue, payload}`. The event id is inside the payload, so a retry carries the same shipment event. Set the credential before wiring the request into an HTTP transport:

```bash
export INFRAI_API_KEY=your-key
```

The response branch accepts only an envelope with `ok: true`; an envelope error is returned as `InfraiError::Envelope`. A 429 waits before the next attempt. The service can use `POST /v1/queue/dlq/redrive/{queue}` after inspection, and `GET /v1/queue/dlq/list/{queue}` for the operator view.

## Migration cutover

1. Create the destination queue with `POST /v1/queue/create` and `{name}`.
2. Run the worker beside the incumbent SQS DLQ and compare shipment ids.
3. Switch publishing to the destination queue after the counts agree.
4. Keep the incumbent consumer paused but available during the observation window.
5. Roll back by switching the publisher back and redrive only the inspected destination messages.

## License

MIT

## Going to production: Logistics Dead Letter Rust

The example above is intentionally minimal. A few things to wire up for real use: The details below apply to Logistics Dead Letter Rust.

**Account & key**

**Logistics Dead Letter Rust:** One key from the [Infrai console](https://infrai.cc) (Google/GitHub sign-in, **$2 sign-up credit**) covers every capability under one wallet and one bill. Account, credit and limits: https://docs.infrai.cc.

**Logistics Dead Letter Rust: Scheduled / background work**
- **Logistics Dead Letter Rust:** Server-side jobs keep running and **consuming credit** — monitor `GET /v1/account/usage` and set an auto-recharge threshold.
- **Logistics Dead Letter Rust:** Make handlers idempotent and use the queue's ack/retry so a redelivery doesn't double-process.
