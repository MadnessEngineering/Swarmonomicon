use anyhow::Result;
use omnispindle::mqtt::{MqttClient, MqttConfig};

pub struct OmniSpindle {
    client: MqttClient,
}

impl OmniSpindle {
    pub fn new(config: MqttConfig) -> Result<Self> {
        let client = MqttClient::new(config.host, config.port, config.client_id)?;
        Ok(Self { client })
    }

    pub fn publish(&self, topic: &str, payload: &str, retain: bool) -> Result<()> {
        self.client.publish(topic, payload, retain)
    }

    pub fn subscribe(&self, topic: &str) -> Result<()> {
        self.client.subscribe(topic)
    }

    pub fn receive_message(&self) -> Result<String> {
        self.client.receive_message()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_omnispindle_integration() {
        let config = MqttConfig {
            host: "localhost".to_string(),
            port: 1883,
            client_id: "swarmonomicon-test".to_string(),
        };

        let omnispindle = OmniSpindle::new(config).unwrap();

        omnispindle.subscribe("test/topic").unwrap();
        omnispindle.publish("test/topic", "Hello, Swarmonomicon!", true).unwrap();

        let msg = omnispindle.receive_message().unwrap();
        assert_eq!(msg, "Hello, Swarmonomicon!");
    }
} 