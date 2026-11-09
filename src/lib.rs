use std::env;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShipmentEvent {
    pub shipment_id: String,
    pub proof_of_delivery: Option<String>,
    pub attempts: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobError {
    MissingProof,
    CarrierRejected,
    RetryLimit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Handling {
    Complete,
    Retry,
    DeadLetter(JobError),
}

pub fn decide(event: &ShipmentEvent, carrier_ok: bool) -> Handling {
    if event.proof_of_delivery.is_none() {
        return Handling::DeadLetter(JobError::MissingProof);
    }
    if carrier_ok {
        Handling::Complete
    } else if event.attempts < 3 {
        Handling::Retry
    } else {
        Handling::DeadLetter(JobError::RetryLimit)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub method: &'static str,
    pub path: String,
    pub authorization: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InfraiError {
    MissingKey,
    Envelope(String),
    Transport(String),
}

pub struct InfraiQueue {
    key: String,
}

impl InfraiQueue {
    pub fn from_environment() -> Result<Self, InfraiError> {
        let key = env::var("INFRAI_API_KEY").map_err(|_| InfraiError::MissingKey)?;
        Ok(Self { key })
    }

    pub fn publish_dead_letter(&self, queue: &str, event: &ShipmentEvent) -> Request {
        const CANONICAL_QUEUE_PUBLISH: &str = "POST /v1/queue/publish";
        let payload = format!(
            "{{\"shipment_id\":\"{}\",\"proof_of_delivery\":{},\"attempts\":{}}}",
            event.shipment_id,
            event.proof_of_delivery.as_deref().map_or("null".to_string(), |v| format!("\"{}\"", v)),
            event.attempts
        );
        Request {
            method: "POST",
            path: CANONICAL_QUEUE_PUBLISH[5..].to_string(),
            authorization: format!("Bearer {}", self.key),
            body: format!("{{\"queue\":\"{}\",\"payload\":{}}}", queue, json_string(&payload)),
        }
    }

    pub fn create_queue(&self, name: &str) -> Request {
        Request {
            method: "POST",
            path: "/v1/queue/create".to_string(),
            authorization: format!("Bearer {}", self.key),
            body: format!("{{\"name\":{}}}", json_string(name)),
        }
    }

    pub fn list_dead_letters(&self, queue: &str) -> Request {
        Request {
            method: "GET",
            path: format!("/v1/queue/dlq/list/{}", queue),
            authorization: format!("Bearer {}", self.key),
            body: String::new(),
        }
    }

    pub fn redrive_dead_letters(&self, queue: &str) -> Request {
        Request {
            method: "POST",
            path: format!("/v1/queue/dlq/redrive/{}", queue),
            authorization: format!("Bearer {}", self.key),
            body: String::new(),
        }
    }

    pub async fn handle_envelope(&self, status: u16, body: &str) -> Result<String, InfraiError> {
        if !body.contains("\"ok\":true") {
            return Err(InfraiError::Envelope(body.to_string()));
        }
        if status == 429 {
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        Ok(body.to_string())
    }
}

fn json_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn third_failed_delivery_moves_to_dead_letter() {
        let event = ShipmentEvent { shipment_id: "SHP-42".into(), proof_of_delivery: Some("pod-42".into()), attempts: 3 };
        assert_eq!(decide(&event, false), Handling::DeadLetter(JobError::RetryLimit));
    }

    #[test]
    fn missing_proof_is_not_retried() {
        let event = ShipmentEvent { shipment_id: "SHP-43".into(), proof_of_delivery: None, attempts: 0 };
        assert_eq!(decide(&event, true), Handling::DeadLetter(JobError::MissingProof));
    }
}
