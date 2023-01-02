

use std::str::FromStr;


use crate::{
    db::RequestStorage,
    models::{Request, ResponseSuccessData, ResponseErrorData, Response},
    supabase_auth::AuthenticatedUser,
};
use anyhow::Context;

use postgrest::Postgrest;
use serde_json::json;

pub struct SupabaseDb {
    connection: Postgrest,
    user: AuthenticatedUser,
}

impl SupabaseDb {
    pub fn new(project_url: &str, key: &str, user: AuthenticatedUser) -> Self {
        let endpoint = format!("{}/rest/v1", project_url);
        Self {
            connection: Postgrest::new(endpoint).insert_header("apikey", key),
            // .insert_header("Authorization", format!("Bearer {}", service_key)),
            user: user,
        }
    }
}

impl SupabaseDb {
    async fn insert_in_table(
        &mut self,
        table_name: &str,
        content: &serde_json::Value,
    ) -> anyhow::Result<u64> {
        self.user.maybe_refresh().await?;
        let r = self
            .connection
            .from(table_name)
            .auth(&self.user.session.access_token)
            .insert(serde_json::to_string(content)?)
            .execute()
            .await?;

        if !r.status().is_success() {
            let r_str = format!("{:?}", &r);
            anyhow::bail!(
                "bad response from supabase {}\n Text {}",
                r_str,
                &r.text().await?
            )
        }

        let response = r.text().await?;
        // dbg!(&response);
        let k = serde_json::Value::from_str(&response)?;
        // dbg!(&response, &k);
        Ok(k.get(0)
            .context("Unexpected empty row")?
            .get("id")
            .context("id not present")?
            .as_u64()
            .context("id is not u64")?)
    }
}

#[async_trait::async_trait]
impl RequestStorage for SupabaseDb {
    async fn store_request(&mut self, req: &Request) -> anyhow::Result<u64> {
        self.insert_in_table(
            "request",
            &json!({
                "content": req.serialize_utf8_body(),
                "user_id": self.user.uid(),
                "created_at": req.created_at.to_rfc3339()
            }),
        )
        .await
    }

    async fn store_response(&mut self, request_id: u64, res: &Response) -> anyhow::Result<()> {
        self.insert_in_table(
            "response",
            &json!({
                "content": res.serialize_utf8_body(),
                "request_id": request_id,
                "user_id": self.user.uid(),
                "created_at": res.created_at().to_rfc3339()
            }),
        )
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
	use super::*;

	#[ignore]
	#[tokio::test]
	async fn my_test() {
		use crate::supabase_auth::*;
		use crate::config::*;
		use bytes::Bytes;

		let mut db = SupabaseDb::new(
			SUPABASE_PROJECT_URL,
			SUPABASE_ANON_KEY,
			create_user().await.unwrap(),
		);
		let r = db
			.store_request(&Request {
				headers: Default::default(),
				uri: "/hello".parse().unwrap(),
				method: http::Method::GET,
				body: Bytes::from_static(b"Just checking"),
				created_at: chrono::Utc::now(),
			})
			.await;
		dbg!(r);

		assert_eq!(1, 2);
	}
}
