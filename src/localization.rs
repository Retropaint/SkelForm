#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct LocalizedStrings {
    pub startup: LocalizationStartup,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct LocalizationStartup {
    pub new: String,
    pub import: String,
    pub samples: String,
    pub empty_recent_files: String,
    pub resources: LocalizationResources,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct LocalizationResources {
    pub user_docs: String,
    pub dev_docs: String,
    pub psd: String,
    pub skf: String,
}
