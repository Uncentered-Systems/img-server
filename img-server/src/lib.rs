use anyhow::Result;
use kinode_process_lib::{
    call_init, 
    kiprintln,
    Address, 
    Message, 
    ProcessId, 
    Response,
    get_blob,
    await_message, 
    get_state,
    logging::{
        error, 
    },
    http::StatusCode,
};
use std::str::FromStr;

pub mod helpers;
pub mod msg_handlers;
pub mod structs;

pub use helpers::*;
pub use msg_handlers::*;
pub use structs::*;

wit_bindgen::generate!({
    path: "target/wit",
    world: "img-server-uncentered-dot-os-v1",
    generate_unused_types: true,
    additional_derives: [serde::Deserialize, serde::Serialize, process_macros::SerdeJsonInto],
});

fn handle_message(_our: &Address, message: Message, state: &mut State) -> Result<()> {

    match message {
        Message::Request { body, source, .. } => handle_request(body, &source, state),
        Message::Response { .. } => Ok(()),
    }
}

fn handle_request(body: Vec<u8>, source: &Address, state: &mut State) -> Result<()> {
    let http_server_address = ProcessId::from_str("http-server:distro:sys")?;
    if source.process.eq(&http_server_address) {
        handle_http_server_request(body, state)
    } else {
        handle_kinode_request(&body, state)
    }
}

fn handle_http_server_request(body: Vec<u8>, state: &mut State) -> anyhow::Result<()> {
    let bytes = get_blob()
        .ok_or(anyhow::anyhow!("Failed to get blob"))?
        .bytes;

    if let Ok(ImgServerRequest::GetImage(uri)) = serde_json::from_slice::<ImgServerRequest>(&bytes){
        kiprintln!("Received a get image request");
        match get_img(uri, state) {
            Ok(img_base64) => {
                send_http_json_response(StatusCode::OK, &img_base64)
            }
            Err(e) => send_http_json_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
        }
     } else {

         let decoded_body = String::from_utf8(body).unwrap_or_else(|_| "Invalid UTF-8".to_string());
         let json_body: serde_json::Value = serde_json::from_str(&decoded_body)?;
         if let Some(_user_agent) = json_body["Http"]["headers"]["user-agent"].as_str() {
         }
         kiprintln!("Uploading image");
         match upload_img(state) {
             Ok(uri) => send_http_json_response(StatusCode::OK, &uri),
             Err(e) => send_http_json_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
         }
     }
}

fn handle_kinode_request(body: &[u8], state: &mut State) -> anyhow::Result<()> {
    let request: ImgServerRequest = serde_json::from_slice(body)?;
    
    let response_body: ImgServerResponse = match request {
        ImgServerRequest::UploadImage => {
            match upload_img(state) {
                Ok(uri) => {
                    kiprintln!("Successfully uploaded image with URI: {:?}", uri);
                    ImgServerResponse::UploadImage(Ok(uri))
                }
                Err(e) => ImgServerResponse::UploadImage(Err(e.to_string())),
            }
        },
        ImgServerRequest::GetImage(get_image_request) => {
            match get_img(get_image_request, state) {
                Ok(img_base64) => ImgServerResponse::GetImage(Ok(img_base64)),
                Err(e) => ImgServerResponse::GetImage(Err(e.to_string())),
            }
        },
    };

    Ok(Response::new().body(response_body).send()?)
}

call_init!(init);
fn init(our: Address) {
    //init_logging(&our, Level::DEBUG, Level::INFO, None).unwrap();
    kiprintln!("begin img-server");

    if let Err(e) = helpers::setup_http_server(&our) {
        kiprintln!("Failed to start server: {}", e);
    }

    let mut state: State = if let Some(state) = get_state() {
        if let Ok(state) = serde_json::from_slice::<State>(&state) {
            kiprintln!("Successfully loaded image state: {:?} images on server", state.images.len());
            state
        } else {
            kiprintln!("Failed to deserialize state, using default");
            State::default()
        }
    } else {
        kiprintln!("No state found, using default");
        State::default()
    };

    loop {
        match await_message() {
            Err(send_error) => error!("got SendError: {send_error}"),
            Ok(message) => {
                if let Err(e) = handle_message(&our, message, &mut state) {
                    error!("got error while handling message: {e:?}");
                }
            }
        }
    }
}
