//! [Application Role Connection Metadata](https://discord.com/developers/docs/resources/application-role-connection-metadata):
//! the values an app reports for Linked Roles

use crate::types::ApplicationRoleConnectionMetadata;

use super::{Body, BotClient, Response, Result, Transport, route::routes};

routes! {
    GET_ROLE_CONNECTION_METADATA = Get "/applications/{application_id}/role-connections/metadata";
    UPDATE_ROLE_CONNECTION_METADATA = Put "/applications/{application_id}/role-connections/metadata";
}

impl<T: Transport> BotClient<T> {
    /// [Get Application Role Connection Metadata Records](https://discord.com/developers/docs/resources/application-role-connection-metadata#get-application-role-connection-metadata-records)
    pub async fn get_role_connection_metadata(
        &self,
    ) -> Result<Response<Vec<ApplicationRoleConnectionMetadata>>> {
        let url = self
            .rest
            .url(GET_ROLE_CONNECTION_METADATA, &[&self.application_id]);
        self.rest
            .call(GET_ROLE_CONNECTION_METADATA, url, Body::NONE, None)
            .await
    }

    /// [Update Application Role Connection Metadata Records](https://discord.com/developers/docs/resources/application-role-connection-metadata#update-application-role-connection-metadata-records):
    /// replaces them all (at most 5)
    pub async fn update_role_connection_metadata(
        &self,
        records: &[ApplicationRoleConnectionMetadata],
    ) -> Result<Response<Vec<ApplicationRoleConnectionMetadata>>> {
        let url = self
            .rest
            .url(UPDATE_ROLE_CONNECTION_METADATA, &[&self.application_id]);
        self.rest
            .call(
                UPDATE_ROLE_CONNECTION_METADATA,
                url,
                Body::Json(&records),
                None,
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use futures::executor::block_on;
    use serde_json::json;

    use super::super::{Method::*, testing::MockTransport};
    use super::*;
    use crate::types::{RoleConnectionMetadataType, Snowflake};

    #[test]
    fn routes() {
        let c = BotClient::new(MockTransport::default(), "t", Snowflake::from_u64(1));
        let record = ApplicationRoleConnectionMetadata {
            t: RoleConnectionMetadataType::IntegerGreaterThanOrEqual,
            key: "level".into(),
            name: "Level".into(),
            name_localizations: None,
            description: "Your level in the game".into(),
            description_localizations: None,
        };
        block_on(c.get_role_connection_metadata()).unwrap();
        block_on(c.update_role_connection_metadata(&[record])).unwrap();
        let t = c.transport();
        assert_eq!(
            t.sent(),
            [
                (Get, "/applications/1/role-connections/metadata".into()),
                (Put, "/applications/1/role-connections/metadata".into()),
            ]
        );
        assert_eq!(
            t.body(1),
            json!([{ "type": 2, "key": "level", "name": "Level", "description": "Your level in the game" }])
        );
    }
}
