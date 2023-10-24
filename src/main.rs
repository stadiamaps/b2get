//! `b2get` is a stupidly simple CLI for downloading files from Backblaze B2.
//! It doesn't do anything else; you'll find none of the rest of the API is covered.
//! It's _just_ a file downloader.
//! It just downloads one file and exits.
//!
//! ## Quickstart
//!
//! 1. Install via `cargo install b2get`.
//! 2. Set `B2_APPLICATION_KEY_ID` and `B2_APPLICATION_KEY` environment variables with your B2 credentials.
//! 3. Invoke via your preferred method giving the bucket name, file name in B2, and path on disk you want to save the file to.
//!
//! ```shell
//! b2get com-your-bucket remote-filename.tar path/to/local-path.tar
//! ```
//!
//! Run with `-h` or `--help` for  all options.

use crate::b2::download_file;
use anyhow::Result;
use clap::Parser;
use reqwest::Client;
use std::time::Duration;

mod b2;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The name of the B2 bucket to fetch the file from
    #[arg(value_name = "BUCKETNAME")]
    bucket_name: String,

    /// The name of the file within the b2 bucket (may contain slashes)
    #[arg(value_name = "FILENAME")]
    filename: String,

    /// The output file path at which to save the result
    #[arg(value_name = "OUTFILE")]
    outfile: String,

    /// Your b2 application key ID
    #[arg(long, env)]
    b2_application_key_id: String,

    /// Your b2 application key
    #[arg(long, env)]
    b2_application_key: String,

    /// Hide the progress indicator
    #[arg(long)]
    no_progress: bool,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let args = Args::parse();

    let client = Client::builder()
        // If it takes 15 sec to connect, something is seriously wrong...
        .connect_timeout(Duration::from_secs(15))
        .build()?;
    let authorization = b2::authorize_account(
        &client,
        &args.b2_application_key_id,
        &args.b2_application_key,
    )
    .await?;

    download_file(
        &client,
        &authorization,
        args.bucket_name,
        args.filename,
        args.outfile,
        args.no_progress,
    )
    .await
}
