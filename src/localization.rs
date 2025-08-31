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
}
