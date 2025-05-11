use multiversx_sc_snippets::imports::*;
use rust_interact_borrower::flash_borrower_cli;

#[tokio::main]
async fn main() {
    flash_borrower_cli().await;
}
