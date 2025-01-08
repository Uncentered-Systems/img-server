use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Default, Debug, Clone, Serialize, Deserialize, process_macros::SerdeJsonInto)]
pub struct State {
    pub images: HashMap<URI, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, process_macros::SerdeJsonInto)]
pub enum ImgServerRequest {
    UploadImage,
    GetImage(URI),
}

#[derive(Debug, Clone, Serialize, Deserialize, process_macros::SerdeJsonInto)]
pub enum ImgServerResponse {
    UploadImage(Result<URI, String>),
    GetImage(Result<String, String>), 
}

pub type URI = String;
