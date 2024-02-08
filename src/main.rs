extern crate google_drive3 as drive3;
use async_recursion::async_recursion;
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use drive3::api::{FileListCall, FileMethods, PermissionMethods};
use drive3::hyper::client::HttpConnector;
use drive3::hyper_rustls::HttpsConnector;
use drive3::{chrono, hyper, hyper_rustls, oauth2, DriveHub, FieldMask};
use drive3::{Error, Result}; // Add this line to import the base64 crate
use tokio;

async fn remove_readonly_permission_from_file(
    permission_methods: &PermissionMethods<'_, HttpsConnector<HttpConnector>>,
    file_id: &str,
) -> Result<()> {
    // ファイルの権限を全て取得
    let permissions = permission_methods
        .list(file_id)
        .add_scope(drive3::api::Scope::Full)
        .doit()
        .await?
        .1
        .permissions
        .unwrap_or_default();

    // typeがanyoneの権限を削除
    for permission in permissions {
        let type_ = permission_methods
            .get(file_id, permission.id.as_ref().unwrap())
            .add_scope(drive3::api::Scope::Full)
            .doit()
            .await?
            .1
            .type_;

        if type_ == Some("anyone".to_string()) {
            permission_methods
                .delete(file_id, permission.id.as_ref().unwrap())
                .add_scope(drive3::api::Scope::Full)
                .doit()
                .await?;
        }
    }
    Ok(())
}

#[async_recursion]
async fn remove_readonly_permission_from_folder(
    file_methods: &FileMethods<'_, HttpsConnector<HttpConnector>>,
    permission_methods: &PermissionMethods<'_, HttpsConnector<HttpConnector>>,
    folder_id: &str,
) -> Result<()> {
    // フォルダ内のファイルを全て取得
    let files = file_methods
        .list()
        .add_scope(drive3::api::Scope::Full)
        .q(&format!("'{}' in parents", folder_id))
        .doit()
        .await?
        .1
        .files
        .unwrap_or_default();

    // フォルダ内のファイルに対して権限削除
    for file in files {
        let id = file.id.clone().unwrap();
        remove_readonly_permission_from_file(&permission_methods, &id).await?;

        // ファイルがフォルダの場合、再帰的に権限削除
        if file.mime_type == Some("application/vnd.google-apps.folder".to_string()) {
            println!("searching: {}", file.name.unwrap_or_default());
            remove_readonly_permission_from_folder(&file_methods, &permission_methods, &id).await?;
        }
    }
    Ok(())
}

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
    let file_methods = hub.files();
    let permission_methods = hub.permissions();
    let root_folder_id =
        std::env::var("GOOGLE_DRIVE_FOLDER_ID").expect("GOOGLE_DRIVE_FOLDER_ID is not set");

    remove_readonly_permission_from_folder(&file_methods, &permission_methods, &root_folder_id)
        .await
        .unwrap();
}
