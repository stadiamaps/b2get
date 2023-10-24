use anyhow::{anyhow, Context};
use futures_util::stream::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{header, Client, Url};
use serde::Deserialize;
use sha1::Digest;
use sha1::Sha1;
use std::cmp::min;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
/// The B2 authorization response model.
///
/// This struct is intentionally minimal and does not include unused features.
pub struct AuthorizeAccountResponse {
    authorization_token: String,
    api_info: ApiInfo,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiInfo {
    storage_api: StorageApi,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageApi {
    download_url: String,
}

pub async fn authorize_account(
    client: &Client,
    b2_key_id: &str,
    b2_key: &str,
) -> reqwest::Result<AuthorizeAccountResponse> {
    let request = client
        .get("https://api.backblazeb2.com/b2api/v3/b2_authorize_account")
        .basic_auth(b2_key_id, Some(b2_key));
    let response = request.send().await?;

    response.json().await
}

pub async fn download_file<P: AsRef<Path>>(
    client: &Client,
    authorization: &AuthorizeAccountResponse,
    bucket_name: String,
    file_name: String,
    output_path: P,
    no_progress: bool,
) -> anyhow::Result<()> {
    let base_url = Url::parse(&authorization.api_info.storage_api.download_url)
        .context("Unable to parse download base URL")?;
    let b2_file_url = base_url
        .join(&format!("file/{bucket_name}/{file_name}"))
        .context("Unable to construct file URL")?;
    let request = client
        .get(b2_file_url.clone())
        .header(
            header::AUTHORIZATION,
            authorization.authorization_token.clone(),
        )
        .header(header::RANGE, "bytes=0-");
    let res = request.send().await?;
    let total_size = res
        .content_length()
        .ok_or(anyhow!("Unable to parse content length"))?;
    let large_file_sha1 = res.headers().get("x-bz-info-large_file_sha1");
    let none_sentinel = String::from("none");
    let sha1_header = res
        .headers()
        .get("X-Bz-Content-Sha1")
        .or(large_file_sha1)
        .ok_or(anyhow!("Missing SHA1 in headers"))?
        .to_str()
        .context("Unable to convert SHA1 header to a string")?
        .to_string();

    let expected_sha1 = if sha1_header == none_sentinel {
        // This API is just a wee bit whack....
        large_file_sha1
            .ok_or(anyhow!("Missing SHA1 in headers"))?
            .to_str()
            .context("Unable to convert SHA1 header to a string")?
            .to_string()
    } else {
        sha1_header
    };

    if expected_sha1 == none_sentinel {
        return Err(anyhow!(
            "Unable to find a SHA1 header in the B2 API response"
        ));
    }

    let progress = if !no_progress {
        let p = ProgressBar::new(total_size);
        p.set_style(ProgressStyle::default_bar()
            .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")?
            .progress_chars("#>-"));
        p.set_message(format!("Downloading {}", b2_file_url));

        Some(p)
    } else {
        None
    };

    let mut file = File::create(output_path)
        .await
        .context("Failed to create file")?;
    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    let mut hasher = Sha1::new();

    while let Some(item) = stream.next().await {
        let chunk = item.context("Error while downloading file")?;
        hasher.update(&chunk);
        file.write_all(&chunk)
            .await
            .or(Err(anyhow!("Error while writing to file")))?;
        let new = min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
        if let Some(p) = &progress {
            p.set_position(new);
        }
    }

    file.sync_all()
        .await
        .or(Err(anyhow!("Error syncing file")))?;

    let hash = hasher.finalize();
    let hash_str = base16ct::lower::encode_string(&hash);

    if hash_str == expected_sha1 {
        if let Some(p) = &progress {
            p.finish_with_message(format!("Finished downloading {b2_file_url}"));
        }

        Ok(())
    } else {
        if let Some(p) = &progress {
            p.finish_with_message(format!("Download failed for {b2_file_url}: SHA1 mismatch."));
        }

        Err(anyhow!("Hash of received data {hash_str} did not match expected SHA1 from b2 ({expected_sha1})"))
    }
}
