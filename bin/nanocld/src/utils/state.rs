use nanocl_stubs::resource::ResourcePartial;
use ntex::rt;
use ntex::http;
use ntex::util::Bytes;
use ntex::channel::mpsc;
use ntex::channel::mpsc::Receiver;
use futures_util::StreamExt;
use futures_util::stream::FuturesUnordered;

use nanocl_utils::http_error::HttpError;

use nanocl_stubs::system::Event;
use nanocl_stubs::cargo_config::CargoConfigPartial;
use nanocl_stubs::state::{
  StateDeployment, StateCargo, StateResources, StateMeta, StateStream,
};

use crate::{utils, repositories};
use crate::models::{StateData, DaemonState};

async fn create_cargoes(
  namespace: &str,
  data: &[CargoConfigPartial],
  version: &str,
  state: &DaemonState,
  sx: mpsc::Sender<Result<Bytes, HttpError>>,
) {
  let _ = sx.send(utils::state::stream_to_bytes(StateStream::Msg(format!(
    "Creating {0} cargoes in namespace: {namespace}",
    data.len(),
  ))));
  data
    .iter()
    .map(|cargo| async {
      let _ = sx.send(utils::state::stream_to_bytes(StateStream::Msg(
        format!("Creating Cargo {0}", cargo.name),
      )));
      let res =
        utils::cargo::create_or_put(namespace, cargo, version, state).await;

      if let Err(err) = res {
        let _ = sx.send(utils::state::stream_to_bytes(StateStream::Error(
          err.to_string(),
        )));
        return Ok(());
      }

      let _ = sx.send(utils::state::stream_to_bytes(StateStream::Msg(
        format!("Created Cargo {0}", cargo.name),
      )));

      let key = utils::key::gen_key(namespace, &cargo.name);
      let state_ptr = state.clone();
      rt::spawn(async move {
        let cargo = utils::cargo::inspect(&key, &state_ptr).await.unwrap();
        let _ = state_ptr
          .event_emitter
          .emit(Event::CargoPatched(Box::new(cargo)))
          .await;
      });
      let res = utils::cargo::start(
        &utils::key::gen_key(namespace, &cargo.name),
        state,
      )
      .await;

      if let Err(err) = res {
        let _ = sx.send(utils::state::stream_to_bytes(StateStream::Error(
          err.to_string(),
        )));
        return Ok(());
      }
      let _ = sx.send(utils::state::stream_to_bytes(StateStream::Msg(
        format!("Started Cargo {0}", cargo.name),
      )));
      let key = utils::key::gen_key(namespace, &cargo.name);
      let state_ptr = state.clone();
      rt::spawn(async move {
        let cargo = utils::cargo::inspect(&key, &state_ptr).await.unwrap();
        let _ = state_ptr
          .event_emitter
          .emit(Event::CargoStarted(Box::new(cargo)))
          .await;
      });
      Ok::<_, HttpError>(())
    })
    .collect::<FuturesUnordered<_>>()
    .collect::<Vec<_>>()
    .await;
}

async fn create_resources(
  data: &[ResourcePartial],
  state: &DaemonState,
  sx: mpsc::Sender<Result<Bytes, HttpError>>,
) {
  let _ = sx.send(utils::state::stream_to_bytes(StateStream::Msg(format!(
    "Creating {0} resources",
    data.len(),
  ))));
  data
    .iter()
    .map(|resource| async {
      let _ = sx.send(utils::state::stream_to_bytes(StateStream::Msg(
        format!("Creating Resource {0}", resource.name),
      )));
      let key = resource.name.to_owned();
      let res =
        utils::resource::create_or_patch(resource.clone(), &state.pool).await;
      if let Err(err) = res {
        let _ = sx.send(utils::state::stream_to_bytes(StateStream::Error(
          err.to_string(),
        )));
        return Ok(());
      }
      let _ = sx.send(utils::state::stream_to_bytes(StateStream::Msg(
        format!("Created Resource {0}", resource.name),
      )));
      let pool = state.pool.clone();
      let event_emitter = state.event_emitter.clone();
      rt::spawn(async move {
        let resource = repositories::resource::inspect_by_key(&key, &pool)
          .await
          .unwrap();
        let _ = event_emitter
          .emit(Event::ResourcePatched(Box::new(resource)))
          .await;
      });
      Ok::<_, HttpError>(())
    })
    .collect::<FuturesUnordered<_>>()
    .collect::<Vec<_>>()
    .await;
}

pub fn stream_to_bytes(state_stream: StateStream) -> Result<Bytes, HttpError> {
  let bytes =
    serde_json::to_string(&state_stream).map_err(|err| HttpError {
      status: http::StatusCode::INTERNAL_SERVER_ERROR,
      msg: format!("unable to serialize state_stream_to_bytes {err}"),
    })?;
  Ok(Bytes::from(bytes + "\r\n"))
}

pub fn parse_state(data: &serde_json::Value) -> Result<StateData, HttpError> {
  let meta =
    serde_json::from_value::<StateMeta>(data.to_owned()).map_err(|err| {
      HttpError {
        status: http::StatusCode::BAD_REQUEST,
        msg: format!("unable to serialize payload {err}"),
      }
    })?;
  match meta.kind.as_str() {
    "Deployment" => {
      let data = serde_json::from_value::<StateDeployment>(data.to_owned())
        .map_err(|err| HttpError {
          status: http::StatusCode::BAD_REQUEST,
          msg: format!("unable to serialize payload {err}"),
        })?;
      Ok(StateData::Deployment(data))
    }
    "Cargo" => {
      let data = serde_json::from_value::<StateCargo>(data.to_owned())
        .map_err(|err| HttpError {
          status: http::StatusCode::BAD_REQUEST,
          msg: format!("unable to serialize payload {err}"),
        })?;
      Ok(StateData::Cargo(data))
    }
    "Resource" => {
      let data = serde_json::from_value::<StateResources>(data.to_owned())
        .map_err(|err| HttpError {
          status: http::StatusCode::BAD_REQUEST,
          msg: format!("unable to serialize payload {err}"),
        })?;
      Ok(StateData::Resource(data))
    }
    _ => Err(HttpError {
      status: http::StatusCode::BAD_REQUEST,
      msg: "unknown type".into(),
    }),
  }
}

pub async fn apply_deployment(
  data: &StateDeployment,
  version: &str,
  state: &DaemonState,
  sx: mpsc::Sender<Result<Bytes, HttpError>>,
) -> Result<(), HttpError> {
  let data = data.clone();
  let version = version.to_owned();
  let state = state.clone();

  // If we have a namespace and it doesn't exist, create it
  // Unless we use `global` as default for the creation of cargoes
  let namespace = if let Some(namespace) = &data.namespace {
    utils::namespace::create_if_not_exists(namespace, &state).await?;
    namespace.to_owned()
  } else {
    "global".into()
  };

  if let Some(cargoes) = data.cargoes {
    create_cargoes(&namespace, &cargoes, &version, &state, sx.clone()).await;
  }

  if let Some(resources) = &data.resources {
    create_resources(resources, &state, sx.clone()).await;
  }

  Ok(())
}

pub async fn apply_cargo(
  data: &StateCargo,
  version: &str,
  state: &DaemonState,
  sx: mpsc::Sender<Result<Bytes, HttpError>>,
) -> Result<(), HttpError> {
  let data = data.clone();
  let version = version.to_owned();
  let state = state.clone();
  // If we have a namespace and it doesn't exist, create it
  // Unless we use `global` as default for the creation of cargoes
  let namespace = if let Some(namespace) = &data.namespace {
    utils::namespace::create_if_not_exists(namespace, &state).await?;
    namespace.to_owned()
  } else {
    "global".into()
  };
  create_cargoes(&namespace, &data.cargoes, &version, &state, sx).await;
  Ok(())
}

pub async fn apply_resource(
  data: &StateResources,
  state: &DaemonState,
  sx: mpsc::Sender<Result<Bytes, HttpError>>,
) -> Result<(), HttpError> {
  let data = data.clone();
  let state = state.clone();
  create_resources(&data.resources, &state, sx).await;
  Ok(())
}

pub async fn revert_deployment(
  data: &StateDeployment,
  state: &DaemonState,
) -> Result<Receiver<Result<Bytes, HttpError>>, HttpError> {
  let (sx, rx) = mpsc::channel::<Result<Bytes, HttpError>>();

  let data = data.clone();
  let state = state.clone();

  rt::spawn(async move {
    let namespace = if let Some(namespace) = &data.namespace {
      namespace.to_owned()
    } else {
      "global".into()
    };

    if let Some(cargoes) = &data.cargoes {
      if sx
        .send(utils::state::stream_to_bytes(StateStream::Msg(format!(
          "Deleting {0} cargoes in namespace {namespace}",
          cargoes.len(),
        ))))
        .is_err()
      {
        log::warn!("User stopped the deployment");
        return Ok(());
      };

      for cargo in cargoes {
        let key = utils::key::gen_key(&namespace, &cargo.name);

        let cargo = match utils::cargo::inspect(&key, &state).await {
          Ok(cargo) => cargo,
          Err(_) => {
            if sx
              .send(utils::state::stream_to_bytes(StateStream::Msg(format!(
                "Skipping Cargo {0} [NOT FOUND]",
                cargo.name
              ))))
              .is_err()
            {
              log::warn!("User stopped the deployment");
              break;
            }
            continue;
          }
        };

        if sx
          .send(utils::state::stream_to_bytes(StateStream::Msg(format!(
            "Deleting Cargo {0}",
            cargo.name
          ))))
          .is_err()
        {
          log::warn!("User stopped the deployment");
          break;
        }
        utils::cargo::delete(&key, Some(true), &state).await?;

        if sx
          .send(utils::state::stream_to_bytes(StateStream::Msg(format!(
            "Deleted Cargo {0}",
            cargo.name
          ))))
          .is_err()
        {
          log::warn!("User stopped the deployment");
          break;
        }

        let state_ptr = state.clone();
        rt::spawn(async move {
          let _ = state_ptr
            .event_emitter
            .emit(Event::CargoDeleted(Box::new(cargo)))
            .await;
          Ok::<_, HttpError>(())
        });
      }
    }

    if let Some(resources) = &data.resources {
      if sx
        .send(utils::state::stream_to_bytes(StateStream::Msg(format!(
          "Deleting {0} resources",
          resources.len(),
        ))))
        .is_err()
      {
        log::warn!("User stopped the deployment");
        return Ok(());
      };

      for resource in resources {
        let key = resource.name.to_owned();
        if sx
          .send(utils::state::stream_to_bytes(StateStream::Msg(format!(
            "Deleting Resource {0}",
            resource.name
          ))))
          .is_err()
        {
          log::warn!("User stopped the deployment");
          break;
        }
        let resource =
          match repositories::resource::inspect_by_key(&key, &state.pool).await
          {
            Ok(resource) => resource,
            Err(_) => {
              if sx
                .send(utils::state::stream_to_bytes(StateStream::Msg(format!(
                  "Skipping Resource {0} [NOT FOUND]",
                  resource.name
                ))))
                .is_err()
              {
                log::warn!("User stopped the deployment");
                return Ok(());
              }
              continue;
            }
          };
        utils::resource::delete(resource.clone(), &state.pool).await?;
        if sx
          .send(utils::state::stream_to_bytes(StateStream::Msg(format!(
            "Deleted Resource {0}",
            resource.name
          ))))
          .is_err()
        {
          log::warn!("User stopped the deployment");
          break;
        }
        let state_ptr = state.clone();
        rt::spawn(async move {
          let _ = state_ptr
            .event_emitter
            .emit(Event::ResourceDeleted(Box::new(resource)))
            .await;
        });
      }
    }
    Ok::<_, HttpError>(())
  });
  Ok(rx)
}

pub async fn revert_cargo(
  data: &StateCargo,
  state: &DaemonState,
) -> Result<Receiver<Result<Bytes, HttpError>>, HttpError> {
  let (sx, rx) = mpsc::channel::<Result<Bytes, HttpError>>();

  let data = data.clone();
  let state = state.clone();

  rt::spawn(async move {
    let namespace = if let Some(namespace) = &data.namespace {
      namespace.to_owned()
    } else {
      "global".into()
    };

    if sx
      .send(utils::state::stream_to_bytes(StateStream::Msg(format!(
        "Deleting {0} cargoes in namespace {namespace}",
        data.cargoes.len(),
      ))))
      .is_err()
    {
      log::warn!("User stopped the deployment");
      return Ok(());
    };

    for cargo in &data.cargoes {
      if sx
        .send(utils::state::stream_to_bytes(StateStream::Msg(format!(
          "Deleting Cargo {0}",
          cargo.name
        ))))
        .is_err()
      {
        log::warn!("User stopped the deployment");
        break;
      }
      let key = utils::key::gen_key(&namespace, &cargo.name);
      let cargo = match utils::cargo::inspect(&key, &state).await {
        Ok(cargo) => cargo,
        Err(_) => {
          if sx
            .send(utils::state::stream_to_bytes(StateStream::Msg(format!(
              "Skipping Cargo {0} [NOT FOUND]",
              cargo.name
            ))))
            .is_err()
          {
            log::warn!("User stopped the deployment");
            break;
          }
          continue;
        }
      };
      utils::cargo::delete(&key, Some(true), &state).await?;
      if sx
        .send(utils::state::stream_to_bytes(StateStream::Msg(format!(
          "Deleted Cargo {0}",
          cargo.name
        ))))
        .is_err()
      {
        log::warn!("User stopped the deployment");
        break;
      }
      let event_emitter = state.event_emitter.clone();
      rt::spawn(async move {
        let _ = event_emitter
          .emit(Event::CargoDeleted(Box::new(cargo)))
          .await;
      });
    }

    Ok::<_, HttpError>(())
  });
  Ok(rx)
}

pub async fn revert_resource(
  data: &StateResources,
  state: &DaemonState,
) -> Result<Receiver<Result<Bytes, HttpError>>, HttpError> {
  let (sx, rx) = mpsc::channel::<Result<Bytes, HttpError>>();

  let data = data.clone();
  let state = state.clone();

  rt::spawn(async move {
    if sx
      .send(utils::state::stream_to_bytes(StateStream::Msg(format!(
        "Deleting {0} resources",
        data.resources.len(),
      ))))
      .is_err()
    {
      log::warn!("User stopped the deployment");
      return Ok(());
    };

    for resource in &data.resources {
      if sx
        .send(utils::state::stream_to_bytes(StateStream::Msg(format!(
          "Deleting Resource {0}",
          resource.name
        ))))
        .is_err()
      {
        log::warn!("User stopped the deployment");
        break;
      }
      let key = resource.name.to_owned();
      let resource =
        match repositories::resource::inspect_by_key(&key, &state.pool).await {
          Ok(resource) => resource,
          Err(_) => {
            if sx
              .send(utils::state::stream_to_bytes(StateStream::Msg(format!(
                "Skipping Resource {0} [NOT FOUND]",
                resource.name
              ))))
              .is_err()
            {
              log::warn!("User stopped the deployment");
              return Ok(());
            }
            continue;
          }
        };
      utils::resource::delete(resource.clone(), &state.pool).await?;
      if sx
        .send(utils::state::stream_to_bytes(StateStream::Msg(format!(
          "Deleted Resource {0}",
          resource.name
        ))))
        .is_err()
      {
        log::warn!("User stopped the deployment");
        break;
      }
      let event_emitter = state.event_emitter.clone();
      rt::spawn(async move {
        let _ = event_emitter
          .emit(Event::ResourceDeleted(Box::new(resource)))
          .await;
      });
    }
    Ok::<_, HttpError>(())
  });
  Ok(rx)
}
