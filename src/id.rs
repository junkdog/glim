use serde::{Deserialize, Deserializer};

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct JobId {
    value: u32,
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ProjectId {
    value: u32,
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub struct PipelineId {
    value: u32,
}

impl ProjectId {
    pub fn new(id: u32) -> Self { Self { value: id } }
}

impl PipelineId {
    pub fn new(id: u32) -> Self { Self { value: id } }
}

impl JobId {
    pub fn new(id: u32) -> Self { Self { value: id } }
}

impl<'de> Deserialize<'de> for ProjectId {
    fn deserialize<D>(deserializer: D) -> Result<ProjectId, D::Error>
        where D: Deserializer<'de>,
    {
        let id = u32::deserialize(deserializer)?;
        Ok(ProjectId::new(id))
    }
}

impl<'de> Deserialize<'de> for PipelineId {
    fn deserialize<D>(deserializer: D) -> Result<PipelineId, D::Error>
        where D: Deserializer<'de>,
    {
        let id = u32::deserialize(deserializer)?;
        Ok(PipelineId::new(id))
    }
}

impl<'de> Deserialize<'de> for JobId {
    fn deserialize<D>(deserializer: D) -> Result<JobId, D::Error>
        where D: Deserializer<'de>,
    {
        let id = u32::deserialize(deserializer)?;
        Ok(JobId::new(id))
    }
}

impl std::fmt::Display for ProjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}


impl std::fmt::Display for PipelineId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl std::fmt::Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}