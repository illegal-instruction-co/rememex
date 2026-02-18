mod search_bar;
mod results_list;
mod status_bar;
mod modal;
mod style;
mod settings_panel;

use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui;
use global_hotkey::GlobalHotKeyEvent;
use tokio::sync::Mutex;
use tray_icon::TrayIconEvent;
use tray_icon::menu::MenuEvent;

use crate::commands;
use crate::config::ConfigState;
use crate::events::{AppEvent, EventReceiver, EventSender};
use crate::i18n::{self, Language};
use crate::state::{
    ContainerListItem, DbState, IndexingProgress, ModelState, RerankerState, SearchResult,
};
use crate::watcher;

use self::modal::ModalState;
use self::settings_panel::SettingsAction;

/// Types de reponses asynchrones envoyees depuis les tasks vers l'UI
enum AsyncResponse {
    SearchResults {
        generation: u64,
        results: Result<Vec<SearchResult>, String>,
    },
    IndexResult(Result<String, String>),
    ClearResult(Result<(), String>),
    ContainerList(Result<(Vec<ContainerListItem>, String), String>),
    ContainerAction(Result<(), String>),
}

pub struct RecallApp {
    // Etat UI
    query: String,
    results: Vec<SearchResult>,
    selected_index: usize,
    status: String,
    status_clear_at: Option<Instant>,
    is_indexing: bool,
    index_progress: Option<IndexingProgress>,

    // Conteneurs
    containers: Vec<ContainerListItem>,
    active_container: String,

    // Panneau de reglages (remplace sidebar)
    settings_open: bool,

    // Modal
    modal: ModalState,

    // i18n
    locale: Language,

    // Etat backend (partage avec les tasks async)
    db_state: Arc<Mutex<DbState>>,
    model_state: Arc<Mutex<ModelState>>,
    reranker_state: Arc<Mutex<RerankerState>>,
    config_state: ConfigState,
    watcher_state: watcher::WatcherState,

    // Canaux d'evenements
    event_tx: EventSender,
    event_rx: EventReceiver,

    // Canal de reponses async
    async_tx: std::sync::mpsc::Sender<AsyncResponse>,
    async_rx: std::sync::mpsc::Receiver<AsyncResponse>,

    // Debounce recherche
    last_query_change: Instant,
    last_searched_query: String,
    search_generation: u64,

    // Runtime Tokio
    runtime: tokio::runtime::Handle,

    // Visibilite fenetre
    visible: bool,
    shown_at: Option<Instant>,
    suppress_hide_until: Option<Instant>,

    // Focus — applique une seule fois a l'affichage (fix : evite le vol de focus chaque frame)
    focus_pending: bool,

    // Auto-resize — detecte les changements de nombre de resultats
    current_n_results: usize,
}

impl RecallApp {
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        db_state: Arc<Mutex<DbState>>,
        model_state: Arc<Mutex<ModelState>>,
        reranker_state: Arc<Mutex<RerankerState>>,
        config_state: ConfigState,
        watcher_state: watcher::WatcherState,
        event_tx: EventSender,
        event_rx: EventReceiver,
        runtime: tokio::runtime::Handle,
        locale: Language,
        initial_containers: Vec<ContainerListItem>,
        initial_active: String,
    ) -> Self {
        let (async_tx, async_rx) = std::sync::mpsc::channel();

        Self {
            query: String::new(),
            results: Vec::new(),
            selected_index: 0,
            status: i18n::ts(locale, "status_model_loading"),
            status_clear_at: None,
            is_indexing: false,
            index_progress: None,

            containers: initial_containers,
            active_container: initial_active,
            settings_open: false,

            modal: ModalState::None,

            locale,

            db_state,
            model_state,
            reranker_state,
            config_state,
            watcher_state,

            event_tx,
            event_rx,

            async_tx,
            async_rx,

            last_query_change: Instant::now(),
            last_searched_query: String::new(),
            search_generation: 0,

            runtime,

            visible: false,
            shown_at: None,
            suppress_hide_until: None,

            focus_pending: false,
            // usize::MAX force un redimensionnement au premier frame
            current_n_results: usize::MAX,
        }
    }

    /// Affiche la fenetre, la centre (style Spotlight) et lui donne le focus.
    fn show_window(&mut self, ctx: &egui::Context) {
        self.shown_at = Some(Instant::now());
        self.suppress_hide_until = None;
        self.focus_pending = true;
        // Force un redimensionnement a la prochaine frame (la taille peut avoir change)
        self.current_n_results = usize::MAX;

        // Centre horizontal, 35% depuis le haut (comportement Spotlight macOS)
        if let Some(monitor) = ctx.input(|i| i.viewport().monitor_size) {
            let win_w = 660.0_f32;
            let win_h = compute_target_height(self.results.len());
            let x = ((monitor.x - win_w) / 2.0).max(0.0);
            let y = (monitor.y * 0.35 - win_h / 2.0).max(0.0);
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(x, y)));
        }

        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }

    fn poll_events(&mut self, ctx: &egui::Context) {
        // Evenements backend (progression indexation, modele charge, etc.)
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                AppEvent::IndexingProgress { current, total, path } => {
                    self.is_indexing = true;
                    self.index_progress = Some(IndexingProgress {
                        current,
                        total,
                        path: path.clone(),
                    });
                    let filename = path.rsplit(['/', '\\']).next().unwrap_or(&path);
                    self.status =
                        i18n::t(self.locale, "status_indexing_file", &[("filename", filename)]);
                }
                AppEvent::IndexingComplete(msg) => {
                    self.status =
                        i18n::t(self.locale, "status_done", &[("message", &msg)]);
                    self.is_indexing = false;
                    self.index_progress = None;
                    self.status_clear_at =
                        Some(Instant::now() + Duration::from_secs(5));
                    self.refresh_containers(ctx);
                }
                AppEvent::ModelLoaded => {
                    self.status.clear();
                    self.is_indexing = false;
                    self.index_progress = None;
                }
                AppEvent::ModelLoadError(err) => {
                    self.status =
                        i18n::t(self.locale, "status_model_error", &[("error", &err)]);
                    self.is_indexing = false;
                    self.index_progress = None;
                }
                AppEvent::RerankerLoaded | AppEvent::RerankerLoadError(_) => {}
            }
            ctx.request_repaint();
        }

        // Reponses async
        while let Ok(resp) = self.async_rx.try_recv() {
            match resp {
                AsyncResponse::SearchResults { generation, results } => {
                    if generation == self.search_generation {
                        match results {
                            Ok(res) => {
                                self.results = res;
                                self.selected_index = 0;
                            }
                            Err(msg) => {
                                if msg.contains("rebuild") || msg.contains("Model changed") {
                                    self.status =
                                        i18n::ts(self.locale, "status_rebuild_needed");
                                } else {
                                    self.status = msg;
                                }
                            }
                        }
                    }
                }
                AsyncResponse::IndexResult(result) => {
                    self.status = match result {
                        Ok(msg) => msg,
                        Err(msg) => msg,
                    };
                    self.is_indexing = false;
                }
                AsyncResponse::ClearResult(result) => {
                    match result {
                        Ok(()) => {
                            self.status = i18n::ts(self.locale, "status_cleared");
                            self.status_clear_at =
                                Some(Instant::now() + Duration::from_secs(4));
                        }
                        Err(msg) => {
                            self.status = msg;
                        }
                    }
                    self.is_indexing = false;
                    self.refresh_containers(ctx);
                }
                AsyncResponse::ContainerList(result) => {
                    if let Ok((list, active)) = result {
                        self.containers = list;
                        self.active_container = active;
                    }
                }
                AsyncResponse::ContainerAction(result) => {
                    if let Err(msg) = result {
                        self.status = msg;
                    }
                    self.refresh_containers(ctx);
                }
            }
            ctx.request_repaint();
        }

        // Expiration du statut
        if let Some(clear_at) = self.status_clear_at {
            if Instant::now() >= clear_at {
                self.status.clear();
                self.status_clear_at = None;
                ctx.request_repaint();
            }
        }

        // Hotkey global
        if let Ok(_event) = GlobalHotKeyEvent::receiver().try_recv() {
            self.visible = !self.visible;
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(self.visible));
            if self.visible {
                self.show_window(ctx);
            }
        }

        // Clic sur l'icone de tray
        if let Ok(TrayIconEvent::Click { .. }) = TrayIconEvent::receiver().try_recv() {
            self.visible = !self.visible;
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(self.visible));
            if self.visible {
                self.show_window(ctx);
            }
        }

        // Menu tray
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            match event.id().0.as_str() {
                "quit" => std::process::exit(0),
                "show" => {
                    self.visible = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    self.show_window(ctx);
                }
                _ => {}
            }
        }

        // Hide-on-unfocus (comportement Spotlight) : masquer apres 300ms debounce
        if self.visible {
            let has_focus = ctx.input(|i| i.focused);
            let debounced = self
                .shown_at
                .map_or(false, |t| t.elapsed() >= Duration::from_millis(300));
            let suppressed = self
                .suppress_hide_until
                .map_or(false, |t| t > Instant::now());
            if !has_focus && debounced && !suppressed && matches!(self.modal, ModalState::None) {
                self.visible = false;
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            }
        }
    }

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        let (down, up, enter, escape, ctrl_o, ctrl_comma) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::ArrowDown),
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::Escape),
                i.modifiers.ctrl && i.key_pressed(egui::Key::O),
                i.modifiers.ctrl && i.key_pressed(egui::Key::Comma),
            )
        });

        if down && !self.results.is_empty() {
            self.selected_index =
                (self.selected_index + 1).min(self.results.len() - 1);
        }
        if up {
            self.selected_index = self.selected_index.saturating_sub(1);
        }
        // Enter ouvre le fichier selectionne (uniquement si aucune modale ouverte)
        if enter && !self.results.is_empty() && matches!(self.modal, ModalState::None) {
            if let Some(result) = self.results.get(self.selected_index) {
                let _ = open::that(&result.path);
            }
        }
        if escape {
            if self.settings_open {
                // Priorite 1 : fermer le panneau de reglages
                self.settings_open = false;
            } else if !self.query.is_empty() {
                // Priorite 2 : vider la requete (les resultats disparaissent apres debounce)
                self.query.clear();
                self.last_query_change = Instant::now();
            } else if matches!(self.modal, ModalState::None) {
                // Priorite 3 : masquer la fenetre
                self.visible = false;
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            }
        }
        if ctrl_o {
            self.pick_folder(ctx);
        }
        if ctrl_comma {
            self.settings_open = !self.settings_open;
        }
    }

    fn maybe_search(&mut self, ctx: &egui::Context) {
        let query = self.query.trim().to_string();

        if query.is_empty() {
            // Effacement des resultats avec debounce 500ms (evite l'UX abrupte)
            if !self.results.is_empty() {
                let elapsed = self.last_query_change.elapsed();
                if elapsed >= Duration::from_millis(500) {
                    self.results.clear();
                    self.last_searched_query.clear();
                } else {
                    ctx.request_repaint_after(Duration::from_millis(500) - elapsed);
                }
            } else {
                self.last_searched_query.clear();
            }
            return;
        }

        if query == self.last_searched_query {
            return;
        }

        let elapsed = self.last_query_change.elapsed();
        if elapsed < Duration::from_millis(300) {
            ctx.request_repaint_after(Duration::from_millis(300) - elapsed);
            return;
        }

        // Lance la recherche
        self.search_generation += 1;
        self.last_searched_query = query.clone();
        let gen = self.search_generation;

        let db = self.db_state.clone();
        let model = self.model_state.clone();
        let reranker = self.reranker_state.clone();
        let config = ConfigState {
            config: self.config_state.config.clone(),
            path: self.config_state.path.clone(),
        };
        let tx = self.async_tx.clone();
        let repaint = ctx.clone();

        self.runtime.spawn(async move {
            let result = commands::search(query, &db, &model, &reranker, &config).await;
            let _ = tx.send(AsyncResponse::SearchResults {
                generation: gen,
                results: result,
            });
            repaint.request_repaint();
        });
    }

    fn pick_folder(&mut self, ctx: &egui::Context) {
        self.settings_open = false;

        let title = i18n::t(
            self.locale,
            "index_folder_title",
            &[("container", &self.active_container)],
        );

        let selected = rfd::FileDialog::new().set_title(&title).pick_folder();
        // Supprime le hide-on-unfocus 500ms apres la fermeture du dialog natif
        self.suppress_hide_until =
            Some(Instant::now() + Duration::from_millis(500));

        if let Some(path) = selected {
            let dir = path.to_string_lossy().to_string();
            self.status = i18n::ts(self.locale, "status_starting");
            self.is_indexing = true;

            let db = self.db_state.clone();
            let model = self.model_state.clone();
            let config = ConfigState {
                config: self.config_state.config.clone(),
                path: self.config_state.path.clone(),
            };
            let ws = self.watcher_state.clone();
            let event_tx = self.event_tx.clone();
            let async_tx = self.async_tx.clone();
            let repaint = ctx.clone();

            self.runtime.spawn(async move {
                let result =
                    commands::index_folder(dir, &db, &model, &config, &ws, event_tx).await;
                let _ = async_tx.send(AsyncResponse::IndexResult(result));
                repaint.request_repaint();
            });
        }
    }

    fn refresh_containers(&self, ctx: &egui::Context) {
        let config = ConfigState {
            config: self.config_state.config.clone(),
            path: self.config_state.path.clone(),
        };
        let tx = self.async_tx.clone();
        let repaint = ctx.clone();
        self.runtime.spawn(async move {
            let result = commands::get_containers(&config).await;
            let _ = tx.send(AsyncResponse::ContainerList(result));
            repaint.request_repaint();
        });
    }

    fn switch_container(&mut self, name: String, ctx: &egui::Context) {
        if name == self.active_container {
            return;
        }
        self.active_container = name.clone();
        self.results.clear();
        self.query.clear();
        self.status =
            i18n::t(self.locale, "status_switched", &[("name", &name)]);
        self.status_clear_at = Some(Instant::now() + Duration::from_secs(3));

        let config = ConfigState {
            config: self.config_state.config.clone(),
            path: self.config_state.path.clone(),
        };
        let db = self.db_state.clone();
        let model = self.model_state.clone();
        let ws = self.watcher_state.clone();
        let event_tx = self.event_tx.clone();
        let tx = self.async_tx.clone();
        let repaint = ctx.clone();
        self.runtime.spawn(async move {
            let result =
                commands::set_active_container(name, &config, &db, &model, &ws, event_tx).await;
            let _ = tx.send(AsyncResponse::ContainerAction(result));
            repaint.request_repaint();
        });
    }

    fn create_container(&mut self, name: String, description: String, ctx: &egui::Context) {
        let config = ConfigState {
            config: self.config_state.config.clone(),
            path: self.config_state.path.clone(),
        };
        let tx = self.async_tx.clone();
        let repaint = ctx.clone();
        self.runtime.spawn(async move {
            let result = commands::create_container(name, description, &config).await;
            let _ = tx.send(AsyncResponse::ContainerAction(result));
            repaint.request_repaint();
        });
    }

    fn delete_container(&mut self, ctx: &egui::Context) {
        if self.active_container == "Default" {
            return;
        }
        let name = self.active_container.clone();
        let config = ConfigState {
            config: self.config_state.config.clone(),
            path: self.config_state.path.clone(),
        };
        let db = self.db_state.clone();
        let tx = self.async_tx.clone();
        let repaint = ctx.clone();
        self.active_container = "Default".to_string();
        self.results.clear();
        self.runtime.spawn(async move {
            let result = commands::delete_container(name, &config, &db).await;
            let _ = tx.send(AsyncResponse::ContainerAction(result));
            repaint.request_repaint();
        });
    }

    fn reset_index(&mut self, ctx: &egui::Context) {
        self.status = i18n::ts(self.locale, "status_clearing");
        self.is_indexing = true;
        self.results.clear();

        let db = self.db_state.clone();
        let config = ConfigState {
            config: self.config_state.config.clone(),
            path: self.config_state.path.clone(),
        };
        let tx = self.async_tx.clone();
        let repaint = ctx.clone();
        self.runtime.spawn(async move {
            let result = commands::reset_index(&db, &config).await;
            let _ = tx.send(AsyncResponse::ClearResult(result));
            repaint.request_repaint();
        });
    }

    fn reindex_all(&mut self, ctx: &egui::Context) {
        self.status = i18n::ts(self.locale, "status_rebuilding");
        self.is_indexing = true;
        self.results.clear();

        let db = self.db_state.clone();
        let model = self.model_state.clone();
        let config = ConfigState {
            config: self.config_state.config.clone(),
            path: self.config_state.path.clone(),
        };
        let event_tx = self.event_tx.clone();
        let tx = self.async_tx.clone();
        let repaint = ctx.clone();
        self.runtime.spawn(async move {
            let result = commands::reindex_all(&db, &model, &config, event_tx).await;
            let _ = tx.send(AsyncResponse::IndexResult(result));
            repaint.request_repaint();
        });
    }
}

impl eframe::App for RecallApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0] // Transparent pour Mica
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_events(ctx);
        self.handle_keyboard(ctx);

        style::apply(ctx);

        // ── Auto-resize : envoie InnerSize uniquement quand le nombre de resultats change ──
        let n = self.results.len();
        if n != self.current_n_results {
            self.current_n_results = n;
            let target_h = compute_target_height(n);
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                egui::vec2(660.0, target_h),
            ));
        }

        let frame = egui::Frame::new()
            .fill(egui::Color32::from_rgba_premultiplied(18, 18, 18, 200))
            .inner_margin(egui::Margin::ZERO)
            .outer_margin(egui::Margin::ZERO)
            .stroke(egui::Stroke::new(
                1.0,
                egui::Color32::from_white_alpha(20),
            ))
            .corner_radius(egui::CornerRadius::same(8u8));

        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            // 1. Barre de recherche
            let placeholder = i18n::t(
                self.locale,
                "search_placeholder",
                &[("container", &self.active_container)],
            );
            let old_query = self.query.clone();
            let mut open_settings = false;
            search_bar::show(
                ui,
                &mut self.query,
                &placeholder,
                &mut self.focus_pending,
                &mut open_settings,
            );
            if open_settings {
                self.settings_open = !self.settings_open;
            }
            if self.query != old_query {
                self.last_query_change = Instant::now();
            }

            // 2. Liste des resultats
            let result_action =
                results_list::show(ui, &self.results, self.selected_index);
            match result_action {
                results_list::ResultAction::None => {}
                results_list::ResultAction::Select(idx) => {
                    self.selected_index = idx;
                }
                results_list::ResultAction::Open(idx) => {
                    if let Some(r) = self.results.get(idx) {
                        let _ = open::that(&r.path);
                    }
                }
            }

            // 3. Barre de statut
            let active_info = self
                .containers
                .iter()
                .find(|c| c.name == self.active_container);
            let folder_count = active_info.map(|i| i.indexed_paths.len()).unwrap_or(0);
            status_bar::show(
                ui,
                &self.status,
                self.is_indexing,
                self.index_progress.as_ref(),
                &self.active_container,
                folder_count,
                self.results.len(),
                self.locale,
            );
        });

        // ── Panneau de reglages (overlay, par-dessus le panel central) ──
        if self.settings_open {
            let action = settings_panel::show(
                ctx,
                &self.containers,
                &self.active_container,
                self.is_indexing,
                self.locale,
            );
            match action {
                SettingsAction::None => {}
                SettingsAction::Close => {
                    self.settings_open = false;
                    self.focus_pending = true;
                }
                SettingsAction::SwitchContainer(name) => {
                    self.settings_open = false;
                    self.switch_container(name, ctx);
                    self.focus_pending = true;
                }
                SettingsAction::CreateContainer => {
                    self.settings_open = false;
                    self.modal = ModalState::CreateContainer {
                        name: String::new(),
                        description: String::new(),
                    };
                }
                SettingsAction::DeleteContainer => {
                    self.settings_open = false;
                    self.modal = ModalState::ConfirmDelete {
                        container_name: self.active_container.clone(),
                    };
                }
                SettingsAction::ClearIndex => {
                    self.settings_open = false;
                    self.modal = ModalState::ConfirmClear {
                        container_name: self.active_container.clone(),
                    };
                }
                SettingsAction::ReindexAll => {
                    self.settings_open = false;
                    let folder_count = self
                        .containers
                        .iter()
                        .find(|c| c.name == self.active_container)
                        .map(|c| c.indexed_paths.len())
                        .unwrap_or(0);
                    self.modal = ModalState::ConfirmReindex {
                        container_name: self.active_container.clone(),
                        folder_count,
                    };
                }
                SettingsAction::AddFolder => {
                    self.settings_open = false;
                    self.pick_folder(ctx);
                }
                SettingsAction::CycleLocale => {
                    self.settings_open = false;
                    self.locale = self.locale.cycle();
                    self.focus_pending = true;
                    let config = self.config_state.config.clone();
                    let path = self.config_state.path.clone();
                    let code = self.locale.code().to_string();
                    self.runtime.spawn(async move {
                        let mut c = config.lock().await;
                        c.locale = code;
                        drop(c);
                        let cs = ConfigState { config, path };
                        let _ = cs.save().await;
                    });
                }
            }
        }

        // ── Modales (overlay au-dessus de tout) ──
        let was_modal_open = !matches!(self.modal, ModalState::None);
        let modal_result = modal::show(ctx, &mut self.modal, self.locale);
        let is_modal_open = !matches!(self.modal, ModalState::None);
        // Quand la modale se ferme, redonner le focus a la barre de recherche
        if was_modal_open && !is_modal_open {
            self.focus_pending = true;
        }
        match modal_result {
            modal::ModalResult::None => {}
            modal::ModalResult::CreateContainer { name, description } => {
                self.create_container(name, description, ctx);
            }
            modal::ModalResult::ConfirmDelete => {
                self.delete_container(ctx);
            }
            modal::ModalResult::ConfirmClear => {
                self.reset_index(ctx);
            }
            modal::ModalResult::ConfirmReindex => {
                self.reindex_all(ctx);
            }
        }

        self.maybe_search(ctx);

        // Repaint intelligent : 50ms si visible, 500ms si cache (reduit CPU x10)
        if self.visible {
            ctx.request_repaint_after(Duration::from_millis(50));
        } else {
            ctx.request_repaint_after(Duration::from_millis(500));
        }
    }
}

/// Calcule la hauteur cible de la fenetre selon le nombre de resultats.
///
/// - search bar : 60px
/// - chaque resultat : 76px (max 6 visibles)
/// - status bar : 28px
fn compute_target_height(n_results: usize) -> f32 {
    const SEARCH_H: f32 = 60.0;
    const RESULT_H: f32 = 76.0;
    const STATUS_H: f32 = 28.0;
    const MAX_VISIBLE: usize = 6;
    let results_h = n_results.min(MAX_VISIBLE) as f32 * RESULT_H;
    SEARCH_H + results_h + STATUS_H
    // 0 resultats : 88px | 6 resultats : 544px
}
