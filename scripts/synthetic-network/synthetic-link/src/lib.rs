use std::ops::RangeInclusive;

use anyhow::bail;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

const DEFAULT_SYNTHETIC_NETWORK: SyntheticNetwork = SyntheticNetwork {
    default_link: DEFAULT_SYNTHETIC_LINK,
    flows: Vec::new(),
};

const DEFAULT_SYNTHETIC_LINK: SyntheticLink = SyntheticLink {
    ingress: DEFAULT_QOS,
    egress: DEFAULT_QOS,
};

const DEFAULT_QOS: QualityOfService = QualityOfService {
    rate: 1000000000,
    loss: StrengthParam::zero(),
    latency: 0,
    jitter: 0,
    jitter_strength: StrengthParam::zero(),
    reorder_packets: false,
};

const DEFAULT_FLOW: Flow = Flow {
    ip: IpPattern::All,
    protocol: Protocol::All,
    port_range: PortRange::all(),
};

#[derive(Serialize, Deserialize, Clone)]
pub struct SyntheticNetwork {
    pub default_link: SyntheticLink,
    pub flows: Vec<SyntheticFlow>,
}

impl Default for SyntheticNetwork {
    fn default() -> Self {
        DEFAULT_SYNTHETIC_NETWORK
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SyntheticLink {
    pub ingress: QualityOfService,
    pub egress: QualityOfService,
}

impl Default for SyntheticLink {
    fn default() -> Self {
        DEFAULT_SYNTHETIC_LINK
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct QualityOfService {
    pub rate: u64,
    pub loss: StrengthParam,
    pub latency: u64,
    pub jitter: u64,
    pub jitter_strength: StrengthParam,
    pub reorder_packets: bool,
}

impl Default for QualityOfService {
    fn default() -> Self {
        DEFAULT_QOS
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SyntheticFlow {
    pub label: NonEmptyString,
    pub flow: Flow,
    pub link: SyntheticLink,
}

impl SyntheticFlow {
    pub fn new(label: NonEmptyString) -> Self {
        Self {
            label,
            flow: DEFAULT_FLOW,
            link: DEFAULT_SYNTHETIC_LINK,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Flow {
    pub ip: IpPattern,
    pub protocol: Protocol,
    #[serde(flatten)]
    pub port_range: PortRange,
}

impl Default for Flow {
    fn default() -> Self {
        DEFAULT_FLOW
    }
}

/// Simple wrapper for the `String` type representing only non-empty strings.
#[derive(Serialize, Deserialize, Clone)]
pub struct NonEmptyString(String);

impl NonEmptyString {
    /// Creates an instance of the NonEmptyString type. Bails if provided value `is_empty`.
    pub fn new(value: String) -> anyhow::Result<Self> {
        if value.is_empty() {
            bail!("`value` must be non-empty");
        }
        Ok(Self(value))
    }
}

impl AsRef<String> for NonEmptyString {
    fn as_ref(&self) -> &String {
        &self.0
    }
}

/// Simple wrapper for the `f64` type representing number in range 0..=1.
#[derive(Serialize, Deserialize, Clone)]
pub struct StrengthParam(f64);

impl Default for StrengthParam {
    fn default() -> Self {
        Self(0.0)
    }
}

impl StrengthParam {
    /// Creates an instance of the `StrengthParam` type. Bails if provided value is not withing 0..=1 range.
    pub fn new(value: f64) -> anyhow::Result<Self> {
        if value > 1.0 {
            bail!("value shouldn't be larger than 1");
        }
        if value < 0.0 {
            bail!("value shouldn't be smaller than 0");
        }
        Ok(Self(value))
    }

    const fn zero() -> Self {
        Self(0.0)
    }
}

impl AsRef<f64> for StrengthParam {
    fn as_ref(&self) -> &f64 {
        &self.0
    }
}

#[derive(Serialize_repr, Deserialize_repr, Clone)]
#[repr(u8)]
pub enum Protocol {
    Icmp = 1,
    Tcp = 6,
    Udp = 17,
    All = 0,
}

/// Simple wrapper for the `RangeInclusive<u16>` type.
#[derive(Serialize, Deserialize, Clone)]
#[serde(try_from = "PortRangeSerde", into = "PortRangeSerde")]
pub struct PortRange(RangeInclusive<u16>);

impl PortRange {
    pub const fn all() -> Self {
        Self(0..=u16::MAX)
    }

    /// Creates an instance of the `PortRange` type. Bails if `port_min > port_max`.
    pub fn new(port_min: u16, port_max: u16) -> anyhow::Result<Self> {
        if port_min > port_max {
            bail!("`port_min` is larger than `port_max`");
        }
        Ok(Self(port_min..=port_max))
    }
}

impl AsRef<RangeInclusive<u16>> for PortRange {
    fn as_ref(&self) -> &RangeInclusive<u16> {
        &self.0
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct PortRangeSerde {
    port_min: u16,
    port_max: u16,
}

impl TryFrom<PortRangeSerde> for PortRange {
    type Error = anyhow::Error;

    fn try_from(value: PortRangeSerde) -> Result<Self, Self::Error> {
        Self::new(value.port_min, value.port_max)
    }
}

impl From<PortRange> for PortRangeSerde {
    fn from(value: PortRange) -> Self {
        PortRangeSerde {
            port_min: *value.0.start(),
            port_max: *value.0.end(),
        }
    }
}

/// Custom type for representing IP patterns, namely `all addresses` or any other specific value.
#[derive(Serialize, Deserialize, Clone)]
#[serde(from = "IpPatternSerde", into = "IpPatternSerde")]
pub enum IpPattern {
    All,
    Ip(u32),
}

#[derive(Serialize, Deserialize, Clone)]
struct IpPatternSerde(u32);

impl From<IpPatternSerde> for IpPattern {
    fn from(value: IpPatternSerde) -> Self {
        match value.0 {
            0 => IpPattern::All,
            ip => IpPattern::Ip(ip),
        }
    }
}

impl From<IpPattern> for IpPatternSerde {
    fn from(value: IpPattern) -> Self {
        let ip = match value {
            IpPattern::All => 0,
            IpPattern::Ip(ip) => ip,
        };
        IpPatternSerde(ip)
    }
}

pub struct SyntheticNetworkClient {
    client: Client,
    url: String,
}

impl SyntheticNetworkClient {
    pub fn new(url: String) -> Self {
        SyntheticNetworkClient {
            client: Client::new(),
            url,
        }
    }

    pub async fn commit_config(&mut self, config: &SyntheticNetwork) -> anyhow::Result<()> {
        let result = self.client.post(&self.url).json(config).send().await;
        Ok(result.map(|_| ())?)
    }

    pub async fn load_config(&mut self) -> anyhow::Result<SyntheticNetwork> {
        let result = self.client.get(&self.url).send().await?;
        Ok(result.json::<SyntheticNetwork>().await?)
    }
}
