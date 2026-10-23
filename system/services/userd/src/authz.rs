//! Autorización polkit de las llamadas D-Bus que modifican estado.

use std::collections::HashMap;

use userd_core::names::PolkitAction;
use zbus::Connection;
use zbus::message::Header;
use zbus_polkit::policykit1::{AuthorityProxy, CheckAuthorizationFlags, Subject};

/// Comprueba la acción polkit para el emisor del mensaje.
pub async fn check(
    connection: &Connection,
    header: &Header<'_>,
    action: PolkitAction,
) -> Result<bool, zbus::Error> {
    let subject = Subject::new_for_message_header(header)
        .map_err(|error| zbus::Error::Failure(error.to_string()))?;
    let proxy = AuthorityProxy::new(connection).await?;
    let result = proxy
        .check_authorization(
            &subject,
            &action.name(),
            &HashMap::new(),
            CheckAuthorizationFlags::AllowUserInteraction.into(),
            "",
        )
        .await?;
    Ok(result.is_authorized)
}
