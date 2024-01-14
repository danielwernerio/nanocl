use bollard_next::container::{RemoveContainerOptions, StopContainerOptions};

use nanocl_error::http::HttpResult;
use nanocl_stubs::{
  system::{NativeEventAction, ObjPsStatusKind},
  process::ProcessKind,
  cargo::CargoKillOptions,
};

use crate::{
  repositories::generic::*,
  models::{
    SystemState, ProcessDb, VmDb, CargoDb, JobDb, JobUpdateDb, ObjPsStatusDb,
    ObjPsStatusUpdate,
  },
};

/// Represent a object that is treated as a process
/// That you can start, restart, stop, logs, etc.
pub trait ObjProcess {
  fn get_process_kind() -> ProcessKind;

  async fn _emit(
    kind_key: &str,
    action: NativeEventAction,
    state: &SystemState,
  ) -> HttpResult<()> {
    match Self::get_process_kind() {
      ProcessKind::Vm => {
        let vm = VmDb::transform_read_by_pk(kind_key, &state.pool).await?;
        state.emit_normal_native_action(&vm, action);
      }
      ProcessKind::Cargo => {
        let cargo =
          CargoDb::transform_read_by_pk(kind_key, &state.pool).await?;
        state.emit_normal_native_action(&cargo, action);
      }
      ProcessKind::Job => {
        JobDb::update_pk(
          kind_key,
          JobUpdateDb {
            updated_at: Some(chrono::Utc::now().naive_utc()),
          },
          &state.pool,
        )
        .await?;
        let job = JobDb::read_by_pk(kind_key, &state.pool)
          .await?
          .try_to_spec()?;
        state.emit_normal_native_action(&job, action);
      }
    }
    Ok(())
  }

  async fn start_process_by_kind_key(
    kind_key: &str,
    state: &SystemState,
  ) -> HttpResult<()> {
    let kind = Self::get_process_kind().to_string();
    log::debug!("{kind} {kind_key}",);
    let current_status =
      ObjPsStatusDb::read_by_pk(kind_key, &state.pool).await?;
    if current_status.actual == ObjPsStatusKind::Running.to_string() {
      log::debug!("{kind} {kind_key} already running",);
      return Ok(());
    }
    let status_update = ObjPsStatusUpdate {
      wanted: Some(ObjPsStatusKind::Running.to_string()),
      prev_wanted: Some(current_status.wanted),
      actual: Some(ObjPsStatusKind::Starting.to_string()),
      prev_actual: Some(current_status.actual),
    };
    log::debug!("{kind} {kind_key} update status");
    ObjPsStatusDb::update_pk(kind_key, status_update, &state.pool).await?;
    Self::_emit(kind_key, NativeEventAction::Starting, state).await?;
    Ok(())
  }

  async fn stop_process_by_kind_key(
    kind_pk: &str,
    state: &SystemState,
  ) -> HttpResult<()> {
    let processes = ProcessDb::read_by_kind_key(kind_pk, &state.pool).await?;
    log::debug!("stop_process_by_kind_pk: {kind_pk}");
    for process in processes {
      let process_state = process.data.state.unwrap_or_default();
      if !process_state.running.unwrap_or_default() {
        return Ok(());
      }
      state
        .docker_api
        .stop_container(
          &process.data.id.unwrap_or_default(),
          None::<StopContainerOptions>,
        )
        .await?;
    }
    Self::_emit(kind_pk, NativeEventAction::Stopping, state).await?;
    Ok(())
  }

  async fn restart_process_by_kind_key(
    pk: &str,
    state: &SystemState,
  ) -> HttpResult<()> {
    let processes = ProcessDb::read_by_kind_key(pk, &state.pool).await?;
    for process in processes {
      state
        .docker_api
        .restart_container(&process.key, None)
        .await?;
    }
    Self::_emit(pk, NativeEventAction::Restart, state).await?;
    Ok(())
  }

  async fn kill_process_by_kind_key(
    pk: &str,
    opts: &CargoKillOptions,
    state: &SystemState,
  ) -> HttpResult<()> {
    let processes = ProcessDb::read_by_kind_key(pk, &state.pool).await?;
    for process in processes {
      state
        .docker_api
        .kill_container(&process.key, Some(opts.clone().into()))
        .await?;
    }
    Ok(())
  }

  /// Delete a process by pk
  async fn del_process_by_pk(
    pk: &str,
    opts: Option<RemoveContainerOptions>,
    state: &SystemState,
  ) -> HttpResult<()> {
    match state.docker_api.remove_container(pk, opts).await {
      Ok(_) => {}
      Err(err) => match &err {
        bollard_next::errors::Error::DockerResponseServerError {
          status_code,
          message: _,
        } => {
          if *status_code != 404 {
            return Err(err.into());
          }
        }
        _ => {
          return Err(err.into());
        }
      },
    };
    ProcessDb::del_by_pk(pk, &state.pool).await?;
    Ok(())
  }
}
