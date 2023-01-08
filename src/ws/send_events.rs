use serde::Serialize;

#[derive(Serialize)]
pub struct EventWrapper<T: Serialize> {
    pub event_type: u32,
    pub data: T,
}

pub trait Event: Serialize {
    fn event_type(&self) -> u32;

    fn serialize_event(&self) -> String {
        let wrapper = EventWrapper {
            event_type: self.event_type(),
            data: self,
        };
        return serde_json::to_string(&wrapper).unwrap();
    }
}

#[derive(Serialize)]
pub struct HelloEvent {
    pub port: u16, // Audio socket port
}
impl Event for HelloEvent {
    fn event_type(&self) -> u32 {
        1
    }
}
