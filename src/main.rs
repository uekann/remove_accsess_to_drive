extern crate google_drive3 as drive3;
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
// use drive3::chrono::naive::serde::serde_from;
use drive3::{chrono, hyper, hyper_rustls, oauth2, DriveHub, FieldMask};
use drive3::{Error, Result}; // Add this line to import the base64 crate
use tokio;

#[tokio::main]
async fn main() {
    // 環境変数「GOOGLE_SERVICE_ACCOUNT_KEY」を取得
    let service_account_key_encoded =
        std::env::var("GOOGLE_SERVICE_ACCOUNT_KEY").expect("GOOGLE_SERVICE_ACCOUNT_KEY is not set");

    // service account keyをbase64でデコード
    let service_account_key_string = URL_SAFE
        .decode(service_account_key_encoded.as_bytes())
        .expect("Failed to decode credentials")
        .iter()
        .map(|&c| c as char)
        .collect::<String>();

    let service_account = oauth2::parse_service_account_key(&service_account_key_string)
        .expect("Failed to parse service account key");

    let auth = oauth2::ServiceAccountAuthenticator::builder(service_account)
        .build()
        .await
        .expect("Failed to create authenticator");

    // ドライブAPIのクライアントを作成
    let client = hyper::Client::builder().build(
        hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .https_or_http()
            .enable_http1()
            .build(),
    );

    let hub = DriveHub::new(client, auth);
    let root_folder_id =
        std::env::var("GOOGLE_DRIVE_FOLDER_ID").expect("GOOGLE_DRIVE_FOLDER_ID is not set");

    let permissions = hub
        .permissions()
        .list(&root_folder_id)
        .add_scope(drive3::api::Scope::Full)
        .doit()
        .await
        .expect("Failed to list permissions")
        .1
        .permissions
        .unwrap_or_default();

    for permission in permissions {
        let id = permission.id.unwrap_or_default();
        let role = permission.role.unwrap_or_default();
        let display_name = hub
            .permissions()
            .get(&root_folder_id, &id)
            .param("fields", "displayName")
            .doit()
            .await
            .expect("Failed to get permission")
            .1
            .display_name
            .unwrap_or("anyone".to_string());

        println!("role: {}, display_name: {}", role, display_name);
    }
}
