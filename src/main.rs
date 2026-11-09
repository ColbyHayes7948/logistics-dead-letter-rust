use logistics_dlq_rust::{decide, Handling, ShipmentEvent};

fn main() {
    let event = ShipmentEvent {
        shipment_id: "SHP-42".to_string(),
        proof_of_delivery: Some("pod-42".to_string()),
        attempts: 3,
    };
    println!("{:?}: {:?}", event, decide(&event, false));
    println!("cutover: publish rejected events to logistics-dead-letter, then redrive after inspection");
    let _ = Handling::Retry;
}
