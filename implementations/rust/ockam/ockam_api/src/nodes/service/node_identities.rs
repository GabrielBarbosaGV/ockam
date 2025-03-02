use ockam::compat::sync::Arc;
use ockam::identity::{Identities, IdentitiesCreation, IdentitiesKeys};
use ockam::identity::{IdentitiesVault, Identity};
use ockam::Result;
use ockam_identity::{IdentitiesRepository, IdentityIdentifier};

use crate::cli_state::traits::StateDirTrait;
use crate::cli_state::CliState;

/// This struct supports identities operation that are either backed by
/// a specific vault or which are using the default vault
pub struct NodeIdentities {
    identities: Arc<Identities>,
    cli_state: CliState,
}

impl NodeIdentities {
    pub fn new(identities: Arc<Identities>, cli_state: CliState) -> NodeIdentities {
        NodeIdentities {
            identities,
            cli_state,
        }
    }

    pub(super) fn identities_vault(&self) -> Arc<dyn IdentitiesVault> {
        self.identities.vault()
    }

    pub(super) fn identities_repository(&self) -> Arc<dyn IdentitiesRepository> {
        self.identities.repository()
    }

    /// Return an identity if it has been created with that name before
    pub(crate) async fn get_identity(&self, identity_name: String) -> Result<Option<Identity>> {
        let repository = self.identities_repository();
        if let Ok(idt_state) = self.cli_state.identities.get(identity_name.as_str()) {
            Ok(repository.get_identity(&idt_state.identifier()).await.ok())
        } else {
            Ok(None)
        }
    }

    pub(crate) async fn get_identifier(&self, identity_name: String) -> Result<IdentityIdentifier> {
        let identity_state = self.cli_state.identities.get(identity_name.as_str())?;
        Ok(identity_state.identifier())
    }

    /// Return an identities creation service backed up by the default vault
    pub(crate) async fn get_default_identities_creation(&self) -> Result<Arc<IdentitiesCreation>> {
        Ok(Arc::new(self.get_identities_creation(None).await?))
    }

    /// Return an identities keys service backed up by the default vault
    pub(crate) async fn get_default_identities_keys(&self) -> Result<Arc<IdentitiesKeys>> {
        Ok(Identities::builder()
            .with_identities_vault(self.identities_vault())
            .build()
            .identities_keys())
    }

    /// Return an identities service, possibly backed by a specific vault
    pub(crate) async fn get_identities(
        &self,
        vault_name: Option<String>,
    ) -> Result<Arc<Identities>> {
        let vault = self.get_identities_vault(vault_name).await?;
        let repository = self.cli_state.identities.identities_repository().await?;
        Ok(Identities::builder()
            .with_identities_vault(vault)
            .with_identities_repository(repository)
            .build())
    }

    /// Return an identities creations service
    pub(crate) async fn get_identities_creation(
        &self,
        vault_name: Option<String>,
    ) -> Result<IdentitiesCreation> {
        let vault = self.get_identities_vault(vault_name).await?;
        Ok(IdentitiesCreation::new(self.identities_repository(), vault))
    }

    /// Return either the default vault or a specific one
    pub(crate) async fn get_identities_vault(
        &self,
        vault_name: Option<String>,
    ) -> Result<Arc<dyn IdentitiesVault>> {
        if let Some(vault) = vault_name {
            let existing_vault = self.cli_state.vaults.get(vault.as_str())?.get().await?;
            Ok(existing_vault)
        } else {
            Ok(self.identities_vault())
        }
    }

    /// Return a service to perform key operations
    pub(crate) async fn get_identities_keys(
        &self,
        vault_name: Option<String>,
    ) -> Result<Arc<IdentitiesKeys>> {
        Ok(Arc::new(IdentitiesKeys::new(
            self.get_identities_vault(vault_name).await?,
        )))
    }
}
