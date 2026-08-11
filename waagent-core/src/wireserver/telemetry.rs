use serde::Serialize;

/// Telemetry structures for XML generation and submission to WireServer
#[derive(Debug, Serialize)]
pub struct TelemetryData {
    #[serde(rename = "@version")]
    pub version: String,
    #[serde(rename = "Provider")]
    pub provider: Provider,
}

#[derive(Debug, Serialize)]
pub struct Provider {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "Event")]
    pub event: Event,
}

#[derive(Debug, Serialize)]
pub struct Event {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "EventData")]
    pub event_data: EventData,
}

#[derive(Debug, Serialize)]
pub struct EventData {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "Param")]
    pub param: Vec<Param>,
}

#[derive(Debug, Serialize)]
pub struct Param {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@value")]
    pub value: String,
}
