use std::path::PathBuf;
use std::sync::Arc;

use drakeflake_config::LayoutConfig;
use drakeflake_flake::Target;
use drakeflake_plan::ManagedState;

use super::cleanup::Optimise;
use super::load::CacheUse;
use super::{
    Capabilities, CheckRequest, Generation, Hub, HubError, Mode, apply, cleanup, commands,
    flake_search, functions, generations, home, images, load, lock, package_sets, persistence,
    portal, rebuild, review, scans, setup, target, try_package, values,
};

impl Hub {
    pub fn log(&self, line: impl Into<String>) {
        self.console().log(line);
    }

    pub fn log_warn(&self, line: impl Into<String>) {
        self.console().warn(line);
    }

    pub fn log_error(&self, line: impl Into<String>) {
        self.console().error(line);
    }

    pub fn target(&self) -> Option<Target> {
        self.session().target()
    }

    pub fn capabilities(&self) -> Capabilities {
        Capabilities::selected(self.session())
    }

    pub fn is_current(&self, generation: Generation) -> bool {
        self.session().is_current(generation)
    }

    pub fn set_layout(&self, layout: LayoutConfig) {
        self.session().set_layout(layout);
    }

    pub fn set_auto_commit(&self, enabled: bool) {
        self.session().set_auto_commit(enabled);
    }

    pub fn managed_file(&self) -> Result<PathBuf, HubError> {
        self.session().managed_file()
    }

    pub fn cancel_apply(&self) {
        if self.operations().cancel() {
            self.log("Cancelling…");
        }
    }
}

impl Hub {
    pub fn start(
        self: &Arc<Self>,
        flake: String,
        preferred_host: Option<String>,
        managed_setting: Option<String>,
        cache_use: CacheUse,
    ) {
        load::start(self, flake, preferred_host, managed_setting, cache_use);
    }

    pub fn restart_load(self: &Arc<Self>) {
        load::restart(self);
    }

    pub fn set_managed(
        self: &Arc<Self>,
        managed_setting: Option<String>,
        hosts: Vec<drakeflake_flake::Host>,
    ) {
        target::set_managed(self, managed_setting, hosts);
    }

    pub fn select_home_user(self: &Arc<Self>, user: String) {
        home::select_user(self, user.into());
    }

    pub fn request_package_set(self: &Arc<Self>, set: String) {
        package_sets::request_package_set(self, set);
    }
}

impl Hub {
    pub fn apply(self: &Arc<Self>, plan: ManagedState, mode: Mode) -> Result<(), HubError> {
        apply::start(self, plan, mode)
    }

    pub fn review_apply(self: &Arc<Self>, plan: ManagedState, mode: Mode) -> Result<(), HubError> {
        review::start(self, plan, mode)
    }

    pub fn restore(self: &Arc<Self>, backup: PathBuf) -> Result<(), HubError> {
        apply::restore(self, backup)
    }

    pub fn rebuild_only(self: &Arc<Self>, mode: Mode) -> Result<(), HubError> {
        rebuild::start(self, mode)
    }

    pub fn update_lock(self: &Arc<Self>, names: Vec<String>) -> Result<(), HubError> {
        lock::update(self, names)
    }

    pub fn generation_action(
        self: &Arc<Self>,
        id: u32,
        action: generations::GenerationAction,
    ) -> Result<(), HubError> {
        generations::act(self, id, action)
    }

    pub fn clean_up(
        self: &Arc<Self>,
        keep: u32,
        days: u32,
        optimise: Optimise,
    ) -> Result<(), HubError> {
        cleanup::clean_up(self, keep, days, optimise)
    }

    pub fn build_image(self: &Arc<Self>, host: &str, format: String) -> Result<(), HubError> {
        images::build_image(self, host, format)
    }

    pub fn track_in_git(self: &Arc<Self>) {
        if let Err(error) = apply::track_in_git(self) {
            self.log_error(error.to_string());
        }
    }

    pub fn load_lock(self: &Arc<Self>, flake: String, check: bool) {
        lock::load(self, flake, check);
    }

    pub fn render_plan(
        &self,
        plan: &ManagedState,
    ) -> Result<(PathBuf, drakeflake_plan::Rendered), Vec<String>> {
        apply::render_plan(self.session(), plan)
    }
}

impl Hub {
    pub fn answer(&self, yes: bool) {
        self.operations().pty().answer(&self.mail().console, yes);
    }

    pub fn answer_secret(&self, secret: &str) {
        self.operations()
            .pty()
            .answer_secret(&self.mail().console, secret);
    }

    pub fn set_terminal_size(&self, columns: u16, rows: u16) {
        self.operations().pty().set_size(columns, rows);
    }
}

impl Hub {
    pub fn load_functions(self: &Arc<Self>) {
        functions::load_functions(self);
    }

    pub fn search_flake(self: &Arc<Self>, query: String) {
        flake_search::search_flake(self, query);
    }

    pub fn pick(
        self: &Arc<Self>,
        purpose: String,
        title: String,
        pick: portal::Pick,
        start: PathBuf,
    ) {
        portal::pick(self, purpose, title, pick, start);
    }

    pub fn save_image(self: &Arc<Self>, file: String, destination: PathBuf) {
        images::save_image(self, file, destination);
    }

    pub fn start_image(self: &Arc<Self>, file: String, format: String) {
        images::start_image(self, file, format);
    }

    pub fn load_image_formats(self: &Arc<Self>, host: String) {
        images::load_image_formats(self, host);
    }

    pub fn find_command(self: &Arc<Self>, command: String) {
        commands::find_command(self, command);
    }

    pub fn commands_ready() -> bool {
        commands::ready()
    }

    pub fn download_command_index(self: &Arc<Self>) {
        commands::download_command_index(self);
    }

    pub fn load_persistence(self: &Arc<Self>) {
        persistence::load_persistence(self);
    }

    pub fn scan_ephemeral(self: &Arc<Self>, subvolume: Option<String>) {
        persistence::scan_ephemeral(self, subvolume);
    }

    pub fn try_package(self: &Arc<Self>, attr: String) -> Result<(), HubError> {
        try_package::try_package(self, attr)
    }

    pub fn request_check(self: &Arc<Self>, request: CheckRequest) {
        values::request_check(self, request);
    }

    pub fn request_package(self: &Arc<Self>, attr: drakeflake_render::PackageAttr) {
        values::request_package(self, attr);
    }

    pub fn request_value(self: &Arc<Self>, name: drakeflake_render::OptionName) {
        values::request_value(self, name);
    }

    pub fn load_generations(self: &Arc<Self>) {
        scans::load_generations(self);
    }

    pub fn load_usage(self: &Arc<Self>) {
        scans::load_usage(self);
    }

    pub fn load_history(self: &Arc<Self>) {
        scans::load_history(self);
    }

    pub fn scan_flakes(self: &Arc<Self>) {
        setup::scan_flakes(self);
    }

    pub fn check_import(self: &Arc<Self>) {
        setup::check_import(self);
    }
}
