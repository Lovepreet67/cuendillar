use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WALVariant {
    Default,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemtableVariant {
    Vector,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemtableMangerVariant {
    Default,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SSTableVariant {
    Default,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BloomVariant {
    Default,
}

impl From<&str> for BloomVariant {
    fn from(value: &str) -> Self {
        match value {
            "default" => BloomVariant::Default,
            _ => BloomVariant::Default,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IndexVariant {
    Default,
}

impl From<&str> for IndexVariant {
    fn from(value: &str) -> Self {
        match value {
            "default" => IndexVariant::Default,
            _ => IndexVariant::Default,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompactionVariant {
    Leveled,
}
