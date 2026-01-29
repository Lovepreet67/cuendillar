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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IndexVariant {
    Default,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompactionVariant {
    Leveled,
}
