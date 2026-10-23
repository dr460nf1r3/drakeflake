use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use qtbridge::{QApp, QmlMethodInvoker, QmlObject};

use crate::hub::Hub;
use crate::models::changes::ChangeModel;
use crate::models::config::SavedConfig;
use crate::models::console::ConsoleModel;
use crate::models::docs::DocsModel;
use crate::models::functions::FunctionModel;
use crate::models::generations::GenerationsModel;
use crate::models::images::ImagesModel;
use crate::models::inputs::InputsModel;
use crate::models::options::{OptionModel, View};
use crate::models::packages::PackageModel;
use crate::models::persistence::PersistenceModel;
use crate::models::relay::Relay;
use crate::models::search::SearchModel;
use crate::models::settings::SettingsModel;
use crate::models::setup::SetupModel;
use crate::models::system::SystemModel;
use crate::startup::Startup;

pub struct Models {
    options: Rc<RefCell<OptionModel>>,
    configured: Rc<RefCell<OptionModel>>,
    home: Rc<RefCell<OptionModel>>,
    tree: Rc<RefCell<OptionModel>>,
    home_tree: Rc<RefCell<OptionModel>>,
    packages: Rc<RefCell<PackageModel>>,
    changes: Rc<RefCell<ChangeModel>>,
    system: Rc<RefCell<SystemModel>>,
    console: Rc<RefCell<ConsoleModel>>,
    setup: Rc<RefCell<SetupModel>>,
    inputs: Rc<RefCell<InputsModel>>,
    images: Rc<RefCell<ImagesModel>>,
    generations: Rc<RefCell<GenerationsModel>>,
    settings: Rc<RefCell<SettingsModel>>,
    functions: Rc<RefCell<FunctionModel>>,
    search: Rc<RefCell<SearchModel>>,
    docs: Rc<RefCell<DocsModel>>,
    persistence: Rc<RefCell<PersistenceModel>>,
}

impl Models {
    pub fn new(hub: &Arc<Hub>, relay: &Arc<Relay>, startup: Startup) -> Self {
        let flake = startup.config.nix.flake_path.clone();
        let config = Rc::new(SavedConfig::new(Arc::clone(hub), startup.config));

        Self {
            options: shared(OptionModel::new(Arc::clone(hub))),
            configured: shared(OptionModel::configured(Arc::clone(hub))),
            home: shared(OptionModel::home(Arc::clone(hub))),
            tree: shared(OptionModel::tree(Arc::clone(hub), View::Tree)),
            home_tree: shared(OptionModel::tree(Arc::clone(hub), View::HomeTree)),
            packages: shared(PackageModel::new(Arc::clone(hub))),
            changes: shared(ChangeModel::new(Arc::clone(hub))),
            system: shared(SystemModel::new(
                Arc::clone(hub),
                Arc::clone(relay),
                Rc::clone(&config),
            )),
            console: shared(ConsoleModel::new(Arc::clone(hub))),
            setup: shared(SetupModel::new(Arc::clone(hub), startup.setup)),
            inputs: shared(InputsModel::new(Arc::clone(hub), Arc::clone(relay), flake)),
            images: shared(ImagesModel::new(Arc::clone(hub))),
            generations: shared(GenerationsModel::new(Arc::clone(hub))),
            settings: shared(SettingsModel::new(
                Arc::clone(hub),
                Arc::clone(relay),
                config,
            )),
            functions: shared(FunctionModel::new(Arc::clone(hub))),
            search: shared(SearchModel::new(Arc::clone(hub))),
            docs: shared(DocsModel::new()),
            persistence: shared(PersistenceModel::new(Arc::clone(hub))),
        }
    }

    pub fn expose(&self, app: &mut QApp) {
        app.set_initial_object("optionModel", Rc::clone(&self.options))
            .set_initial_object("configuredModel", Rc::clone(&self.configured))
            .set_initial_object("homeModel", Rc::clone(&self.home))
            .set_initial_object("treeModel", Rc::clone(&self.tree))
            .set_initial_object("homeTreeModel", Rc::clone(&self.home_tree))
            .set_initial_object("packageModel", Rc::clone(&self.packages))
            .set_initial_object("changeModel", Rc::clone(&self.changes))
            .set_initial_object("systemModel", Rc::clone(&self.system))
            .set_initial_object("consoleModel", Rc::clone(&self.console))
            .set_initial_object("setupModel", Rc::clone(&self.setup))
            .set_initial_object("inputsModel", Rc::clone(&self.inputs))
            .set_initial_object("imagesModel", Rc::clone(&self.images))
            .set_initial_object("generationsModel", Rc::clone(&self.generations))
            .set_initial_object("settingsModel", Rc::clone(&self.settings))
            .set_initial_object("functionModel", Rc::clone(&self.functions))
            .set_initial_object("searchModel", Rc::clone(&self.search))
            .set_initial_object("docsModel", Rc::clone(&self.docs))
            .set_initial_object("persistenceModel", Rc::clone(&self.persistence));
    }

    pub fn attach(&self, hub: &Hub, relay: &Relay, start: bool) {
        let mail = hub.mail();
        let mut busy = 0_u32;
        let mut connect = |invoker: Option<QmlMethodInvoker>, attach: &dyn Fn(QmlMethodInvoker)| {
            invoker.map_or_else(|| busy += 1_u32, attach);
        };

        connect(invoker(&self.options), &|it| mail.options.attach(it));
        connect(invoker(&self.configured), &|it| mail.configured.attach(it));
        connect(invoker(&self.home), &|it| mail.home.attach(it));
        connect(invoker(&self.tree), &|it| mail.tree.attach(it));
        connect(invoker(&self.home_tree), &|it| mail.home_tree.attach(it));
        connect(invoker(&self.packages), &|it| mail.packages.attach(it));
        connect(invoker(&self.changes), &|it| mail.changes.attach(it));
        connect(invoker(&self.system), &|it| mail.system.attach(it));
        connect(invoker(&self.system), &|it| relay.system.attach(it));
        connect(invoker(&self.console), &|it| mail.console.attach(it));
        connect(invoker(&self.setup), &|it| mail.setup.attach(it));
        connect(invoker(&self.inputs), &|it| mail.inputs.attach(it));
        connect(invoker(&self.images), &|it| mail.images.attach(it));
        connect(invoker(&self.generations), &|it| {
            mail.generations.attach(it);
        });
        connect(invoker(&self.functions), &|it| mail.functions.attach(it));
        connect(invoker(&self.search), &|it| mail.search.attach(it));
        connect(invoker(&self.persistence), &|it| {
            mail.persistence.attach(it);
        });

        if start {
            // Queued: runs once the event loop is up, independent of QML.
            connect(invoker(&self.system), &|it| {
                it.invoke_method("start");
            });
        } else {
            log::info!("first start without a usable flake: showing the setup assistant");
        }

        if busy > 0_u32 {
            log::warn!("{busy} models were busy and not connected to the hub");
        }
    }
}

fn shared<T>(model: T) -> Rc<RefCell<T>> {
    Rc::new(RefCell::new(model))
}

fn invoker<T>(model: &RefCell<T>) -> Option<QmlMethodInvoker>
where
    T: QmlObject,
{
    model
        .try_borrow()
        .ok()
        .map(|model| model.get_qml_method_invoker())
}
