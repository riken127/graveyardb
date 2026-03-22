use graveyar_db::api::event_store_client::EventStoreClient;
use graveyar_db::api::{AppendEventRequest, Event};
use std::time::Instant;

use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "http://127.0.0.1:50051";
    println!("Starting stress test against {}", addr);

    let concurrency = 50;
    let events_per_worker = 1000;
    let total_events = concurrency * events_per_worker;

    let start = Instant::now();
    let mut handles = Vec::new();

    for i in 0..concurrency {
        let uri = addr.to_string();
        handles.push(tokio::spawn(async move {
            let mut client = EventStoreClient::connect(uri).await.unwrap();
            let stream_id = format!("stress-stream-{}", i);

            for j in 0..events_per_worker {
                let event = Event {
                    id: Uuid::now_v7().to_string(),
                    event_type: "StressTestEvent".to_string(),
                    payload: format!("{{\"worker\": {}, \"seq\": {}}}", i, j).into_bytes(),
                    timestamp: 0,
                    metadata: std::collections::HashMap::new(),
                };

                let req = AppendEventRequest {
                    stream_id: stream_id.clone(),
                    events: vec![event],
                    expected_version: -1, // Sentinel for "append regardless of current version"
                    is_forwarded: false,
                };

                if let Err(e) = client.append_event(req).await {
                    eprintln!("Worker {} failed append: {}", i, e);
                }
            }
        }));
    }

    for h in handles {
        h.await?;
    }

    let duration = start.elapsed();
    let seconds = duration.as_secs_f64();
    let tps = total_events as f64 / seconds;

    println!("Stress test completed.");
    println!("Total Events: {}", total_events);
    println!("Duration: {:.2}s", seconds);
    println!("Throughput: {:.2} events/sec", tps);

    Ok(())
}
