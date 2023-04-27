use dialoguer::Confirm;
use dialoguer::theme::ColorfulTheme;
use futures::StreamExt;
use bollard_next::exec::CreateExecOptions;

use nanocl_utils::io_error::{IoError, IoResult};
use nanocld_client::NanocldClient;
use nanocld_client::stubs::cargo::{OutputKind, CargoDeleteQuery, CargoLogQuery};

use crate::utils::print::{print_yml, print_table};
use crate::models::{
  CargoArgs, CargoCreateOpts, CargoCommands, CargoRemoveOpts, CargoRow,
  CargoStartOpts, CargoStopOpts, CargoPatchOpts, CargoInspectOpts,
  CargoExecOpts, CargoHistoryOpts, CargoResetOpts, CargoLogsOpts, CargoRunOpts,
};

use super::cargo_image::{self, exec_cargo_image_create};

/// Execute cargo command
///
/// ## Arguments
/// * [client](NanocldClient) - Nanocl client
/// * [args](CargoArgs) - Cargo arguments
/// * [options](CargoCommands) - Cargo command
///
/// ## Returns
/// * [Result](Result) - Result of the operation
///   * [Ok](Ok) - Operation was successful
///   * [Err](IoError) - Operation failed
///
async fn exec_cargo_create(
  client: &NanocldClient,
  args: &CargoArgs,
  options: &CargoCreateOpts,
) -> IoResult<()> {
  let cargo = options.clone().into();
  let item = client.create_cargo(&cargo, args.namespace.clone()).await?;
  println!("{}", &item.key);
  Ok(())
}

async fn exec_cargo_rm(
  client: &NanocldClient,
  args: &CargoArgs,
  options: &CargoRemoveOpts,
) -> IoResult<()> {
  if !options.skip_confirm {
    let result = Confirm::with_theme(&ColorfulTheme::default())
      .with_prompt(format!("Delete cargoes {}?", options.names.join(",")))
      .default(false)
      .interact();
    match result {
      Ok(true) => {}
      _ => {
        return Err(IoError::interupted("Cargo remove", "interupted by user"))
      }
    }
  }
  let query = CargoDeleteQuery {
    namespace: args.namespace.clone(),
    force: Some(options.force),
  };
  for name in &options.names {
    client.delete_cargo(name, &query).await?;
  }
  Ok(())
}

async fn exec_cargo_ls(
  client: &NanocldClient,
  args: &CargoArgs,
) -> IoResult<()> {
  let items = client.list_cargo(args.namespace.clone()).await?;

  let rows = items
    .into_iter()
    .map(CargoRow::from)
    .collect::<Vec<CargoRow>>();
  print_table(rows);
  Ok(())
}

async fn exec_cargo_start(
  client: &NanocldClient,
  args: &CargoArgs,
  options: &CargoStartOpts,
) -> IoResult<()> {
  client
    .start_cargo(&options.name, args.namespace.clone())
    .await?;
  Ok(())
}

async fn exec_cargo_stop(
  client: &NanocldClient,
  args: &CargoArgs,
  options: &CargoStopOpts,
) -> IoResult<()> {
  for name in &options.names {
    client.stop_cargo(name, args.namespace.clone()).await?;
  }
  Ok(())
}

async fn exec_cargo_patch(
  client: &NanocldClient,
  args: &CargoArgs,
  options: &CargoPatchOpts,
) -> IoResult<()> {
  let cargo = options.clone().into();
  client
    .patch_cargo(&options.name, cargo, args.namespace.clone())
    .await?;
  Ok(())
}

async fn exec_cargo_inspect(
  client: &NanocldClient,
  args: &CargoArgs,
  options: &CargoInspectOpts,
) -> IoResult<()> {
  let cargo = client
    .inspect_cargo(&options.name, args.namespace.clone())
    .await?;
  print_yml(cargo)?;
  Ok(())
}

async fn exec_cargo_exec(
  client: &NanocldClient,
  args: &CargoArgs,
  options: &CargoExecOpts,
) -> IoResult<()> {
  let exec: CreateExecOptions = options.clone().into();
  let mut stream = client
    .exec_cargo(&options.name, exec, args.namespace.clone())
    .await?;

  while let Some(output) = stream.next().await {
    let output = output?;
    match output.kind {
      OutputKind::StdOut => {
        print!("{}", &output.data);
      }
      OutputKind::StdErr => {
        eprint!("{}", output.data);
      }
      OutputKind::StdIn => println!("TODO: StdIn {}", &output.data),
      OutputKind::Console => print!("{}", &output.data),
    }
  }

  Ok(())
}

async fn exec_cargo_history(
  client: &NanocldClient,
  args: &CargoArgs,
  opts: &CargoHistoryOpts,
) -> IoResult<()> {
  let histories = client
    .list_history_cargo(&opts.name, args.namespace.clone())
    .await?;

  print_yml(histories)?;
  Ok(())
}

async fn exec_cargo_logs(
  client: &NanocldClient,
  args: &CargoArgs,
  options: &CargoLogsOpts,
) -> IoResult<()> {
  let query = CargoLogQuery {
    namespace: args.namespace.clone(),
    tail: options.tail.clone(),
    since: options.since,
    until: options.until,
    follow: Some(options.follow),
    timestamps: Some(options.timestamps),
    stderr: None,
    stdout: None,
  };
  let mut stream = client.logs_cargo(&options.name, &query).await?;
  while let Some(log) = stream.next().await {
    let log = match log {
      Ok(log) => log,
      Err(e) => {
        eprintln!("Error: {e}");
        break;
      }
    };
    match log.kind {
      OutputKind::StdOut => {
        print!("{}", &log.data);
      }
      OutputKind::StdErr => {
        eprint!("{}", log.data);
      }
      OutputKind::StdIn => println!("TODO: StdIn {}", &log.data),
      OutputKind::Console => print!("{}", &log.data),
    }
  }
  Ok(())
}

async fn exec_cargo_reset(
  client: &NanocldClient,
  args: &CargoArgs,
  opts: &CargoResetOpts,
) -> IoResult<()> {
  let cargo = client
    .reset_cargo(&opts.name, &opts.history_id, args.namespace.clone())
    .await?;
  print_yml(cargo)?;
  Ok(())
}

async fn exec_cargo_run(
  client: &NanocldClient,
  args: &CargoArgs,
  opts: &CargoRunOpts,
) -> IoResult<()> {
  // Image is not existing so we donwload it
  if client.inspect_cargo_image(&opts.image).await.is_err() {
    exec_cargo_image_create(client, &opts.image).await?;
  }

  let cargo = client
    .create_cargo(&opts.clone().into(), args.namespace.clone())
    .await?;

  client
    .start_cargo(&cargo.name, Some(cargo.namespace_name))
    .await?;

  Ok(())
}

pub async fn exec_cargo(
  client: &NanocldClient,
  args: &CargoArgs,
) -> IoResult<()> {
  match &args.commands {
    CargoCommands::List => exec_cargo_ls(client, args).await,
    CargoCommands::Create(options) => {
      exec_cargo_create(client, args, options).await
    }
    CargoCommands::Remove(options) => {
      exec_cargo_rm(client, args, options).await
    }
    CargoCommands::Image(options) => {
      cargo_image::exec_cargo_image(client, options).await
    }
    CargoCommands::Start(options) => {
      exec_cargo_start(client, args, options).await
    }
    CargoCommands::Stop(options) => {
      exec_cargo_stop(client, args, options).await
    }
    CargoCommands::Patch(options) => {
      exec_cargo_patch(client, args, options).await
    }
    CargoCommands::Inspect(options) => {
      exec_cargo_inspect(client, args, options).await
    }
    CargoCommands::Exec(options) => {
      exec_cargo_exec(client, args, options).await
    }
    CargoCommands::History(opts) => {
      exec_cargo_history(client, args, opts).await
    }
    CargoCommands::Reset(options) => {
      exec_cargo_reset(client, args, options).await
    }
    CargoCommands::Logs(options) => {
      exec_cargo_logs(client, args, options).await
    }
    CargoCommands::Run(options) => exec_cargo_run(client, args, options).await,
  }
}
