use ockam_api::cli_state::StateDirTrait;
use tauri::{AppHandle, CustomMenuItem, SystemTray, SystemTrayMenu, SystemTrayMenuItem, Wry};
use tracing::log::{error, info};

use ockam_command::CommandGlobalOpts;

use crate::enroll::EnrollActions;
use crate::quit::QuitActions;
use crate::tcp::outlet::TcpOutletActions;
use crate::Result;

/// Make a full system tray
pub fn make_system_tray(options: &CommandGlobalOpts) -> SystemTray {
    SystemTray::new().with_menu(SystemTrayMenuBuilder::default(options))
}

/// Create the system tray with all the major functions.
/// Separate groups of related functions with a native separator.
pub struct SystemTrayMenuBuilder {
    enroll: EnrollActions,
    tcp: TcpOutletActions,
    quit: QuitActions,
}

impl SystemTrayMenuBuilder {
    /// Create the default system tray menu with the basic elements (i.e. without list items).
    pub fn default(options: &CommandGlobalOpts) -> SystemTrayMenu {
        Self::init(options).build(options)
    }

    pub fn init(options: &CommandGlobalOpts) -> Self {
        let enroll = EnrollActions::new(options);
        let tcp = TcpOutletActions::new(options);
        let quit = QuitActions::new();
        Self { enroll, tcp, quit }
    }

    /// Create a `SystemTrayMenu` instance, adding the menu items in the expected order.
    pub fn build(self, options: &CommandGlobalOpts) -> SystemTrayMenu {
        match options.state.projects.default() {
            Ok(_) => Self::init(options).after_enroll(),
            Err(_) => Self::init(options).before_enroll(),
        }
    }

    fn after_enroll(self) -> SystemTrayMenu {
        SystemTrayMenu::new()
            .add_menu_items(&[self.enroll.enroll])
            .add_menu_items(&[self.enroll.please_enroll])
            .add_native_item(SystemTrayMenuItem::Separator)
            .add_menu_items(&self.tcp.menu_items)
            .add_native_item(SystemTrayMenuItem::Separator)
            .add_menu_items(&[self.enroll.reset, self.quit.quit])
    }

    fn before_enroll(self) -> SystemTrayMenu {
        SystemTrayMenu::new()
            .add_menu_items(&[self.enroll.enroll])
            .add_native_item(SystemTrayMenuItem::Separator)
            .add_menu_items(&[self.enroll.please_enroll])
            .add_native_item(SystemTrayMenuItem::Separator)
            .add_menu_items(&[self.quit.quit])
    }

    /// Refresh the system tray menu with the latest state, including all list items.
    pub async fn refresh(app: &AppHandle<Wry>, options: &CommandGlobalOpts) -> Result<()> {
        info!("refreshing the menu");
        let menu = Self::get_full_menu(app, options).await.unwrap_or_else(|e| {
            error!("{:?}", e);
            Self::default(options)
        });
        app.tray_handle().set_menu(menu)?;
        Ok(())
    }

    async fn get_full_menu(
        app: &AppHandle<Wry>,
        options: &CommandGlobalOpts,
    ) -> Result<SystemTrayMenu> {
        let enroll = EnrollActions::new(options);
        let tcp = TcpOutletActions::full(app, options).await?;
        let quit = QuitActions::new();
        let menu = Self { enroll, tcp, quit }.build(options);
        Ok(menu)
    }
}

/// This trait provides a way to add a list of
/// custom menu items to the SystemTray so that we
/// can define the behaviour of those items in separate modules.
pub(crate) trait SystemTrayMenuItems {
    fn add_menu_items(self, items: &[CustomMenuItem]) -> Self;
}

impl SystemTrayMenuItems for SystemTrayMenu {
    fn add_menu_items(self, items: &[CustomMenuItem]) -> Self {
        let mut tm = self;
        for item in items.iter() {
            tm = tm.add_item(item.clone());
        }
        tm
    }
}
