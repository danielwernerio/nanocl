use nanocl_utils::io_error::IoResult;
use nanocld_client::NanocldClient;

use crate::{version, config::CommandConfig};

async fn print_version(client: &NanocldClient) -> IoResult<()> {
  println!("=== [nanocli] ===");
  version::print_version();

  let daemon_version = client.get_version().await?;
  println!("=== [nanocld] ===");
  println!(
    "Arch: {}\nChannel: {}\nVersion: {}\nCommit ID: {}",
    daemon_version.arch,
    daemon_version.channel,
    daemon_version.version,
    daemon_version.commit_id
  );

  Ok(())
}

pub async fn exec_version(
  cmd_conf: &CommandConfig<Option<u8>>,
) -> IoResult<()> {
  let client = &cmd_conf.client;
  print_version(client).await?;
  Ok(())
}
