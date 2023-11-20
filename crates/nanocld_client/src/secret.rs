use nanocl_error::http_client::HttpClientResult;

use nanocl_stubs::secret::{Secret, SecretPartial, SecretUpdate, SecretQuery};

use super::http_client::NanocldClient;

impl NanocldClient {
  /// ## Default path for secrets
  const SECRET_PATH: &'static str = "/secrets";

  /// ## List secrets
  ///
  /// List existing secrets in the system.
  ///
  /// ## Arguments
  ///
  /// * [query](Option) - The optional [query](SecretQuery)
  ///
  /// ## Return
  ///
  /// [HttpClientResult](HttpClientResult) containing a [Vec](Vec) of [Secret](Secret)
  ///
  /// ## Example
  ///
  /// ```no_run,ignore
  /// use nanocld_client::NanocldClient;
  ///
  /// let client = NanocldClient::connect_to("http://localhost:8585", None);
  /// let res = client.list_secret(None).await;
  /// ```
  ///
  pub async fn list_secret(
    &self,
    query: Option<&SecretQuery>,
  ) -> HttpClientResult<Vec<Secret>> {
    let res = self.send_get(Self::SECRET_PATH, query).await?;
    Self::res_json(res).await
  }

  /// ## Create secret
  ///
  /// ## Arguments
  ///
  /// * [secret](SecretPartial) - The secret to create
  ///
  /// ## Return
  ///
  /// [HttpClientResult](HttpClientResult) containing a [Secret](Secret)
  ///
  pub async fn create_secret(
    &self,
    item: &SecretPartial,
  ) -> HttpClientResult<Secret> {
    let res = self
      .send_post(Self::SECRET_PATH, Some(item), None::<String>)
      .await?;
    Self::res_json(res).await
  }

  /// ## Patch secret
  ///
  /// Patch a secret by it's key to update it with new data
  ///
  /// ## Arguments
  ///
  /// * [secret](SecretUpdate) - The key of the secret to create
  ///
  /// ## Return
  ///
  /// [HttpClientResult](HttpClientResult) containing a [Secret](Secret)
  ///
  pub async fn patch_secret(
    &self,
    item: &SecretUpdate,
  ) -> HttpClientResult<Secret> {
    let res = self
      .send_patch(Self::SECRET_PATH, Some(item), None::<String>)
      .await?;
    Self::res_json(res).await
  }

  /// ## Inspect secret
  ///
  /// Inspect a secret by it's key to get more information about it
  ///
  /// ## Arguments
  ///
  /// * [key](str) - The key of the secret to inspect
  ///
  /// ## Return
  ///
  /// [HttpClientResult](HttpClientResult) containing a [Secret](Secret)
  ///
  /// ## Example
  ///
  /// ```no_run,ignore
  /// use nanocld_client::NanocldClient;
  ///
  /// let client = NanocldClient::connect_to("http://localhost:8585", None);
  /// let secret = client.inspect_secret("my-secret").await?;
  /// ```
  ///
  pub async fn inspect_secret(&self, key: &str) -> HttpClientResult<Secret> {
    let res = self
      .send_get(
        &format!("{}/{key}/inspect", Self::SECRET_PATH),
        None::<String>,
      )
      .await?;
    Self::res_json(res).await
  }

  /// ## Delete a secret
  ///
  /// Delete a [secret](Secret) by it's key
  ///
  /// ## Arguments
  ///
  /// * [key](str) - The key of the [secret](Secret) to delete
  ///
  /// ## Example
  ///
  /// ```no_run,ignore
  /// use nanocld_client::NanocldClient;
  ///
  /// let client = NanocldClient::connect_to("http://localhost:8585", None);
  /// client.delete_secret("my-secret").await?;
  /// ```
  ///
  pub async fn delete_secret(&self, key: &str) -> HttpClientResult<()> {
    self
      .send_delete(&format!("{}/{key}", Self::SECRET_PATH), None::<String>)
      .await?;
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[ntex::test]
  async fn basic() {
    const SECRET_KEY: &str = "secret-test";
    let client =
      NanocldClient::connect_to("http://ndaemon.nanocl.internal:8585", None);
    client.list_secret(None).await.unwrap();
    let secret = SecretPartial {
      key: SECRET_KEY.to_owned(),
      kind: "generic".to_owned(),
      data: serde_json::json!({"key": "value"}),
      metadata: None,
      immutable: None,
    };
    let secret = client.create_secret(&secret).await.unwrap();
    assert_eq!(secret.key, SECRET_KEY);
    let secret = client.inspect_secret(SECRET_KEY).await.unwrap();
    assert_eq!(secret.key, SECRET_KEY);
    client.delete_secret(SECRET_KEY).await.unwrap();
  }
}
